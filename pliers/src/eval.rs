//! Everything related to the evaluation function that gets tuned.
//!
//! To use the tuner for your eval, you need to implement the [`Eval`] trait
//! and its super trait, [`WeightsInterpretation`].

use crate::eval::Direction::{Down, Up};
use crate::eval::EvalScale::{InitialWeights, Scale};
use crate::gd::{
    cp_eval_for_weights, cp_to_wr, sample_loss, Batch, Datapoint, DefaultOptimizer, Float,
    LossGradient, Optimizer, Outcome, ScalingFactor, Weight, Weights,
};
use crate::load_data::Filter;
use crate::trace::TraceTrait;
use derive_more::Display;
use gears::general::board::Board;
use std::fmt::Formatter;

pub mod chess;

/// Returns a [`Vec`] of [`bool`] where the ith entry is [`true`] iff the absolute difference between `weights[i]` and
/// `old_weights[i]` is at least `threshold`. If `threshold` is negative, this is instead [`true`] iff the rounded
/// values differ (i.e. `0.49` and `0.51` would be different, but not `0.51` and `1.23`).
#[must_use]
pub fn changed_at_least(threshold: Float, weights: &Weights, old_weights: &[Weight]) -> Vec<bool> {
    let mut res = vec![false; weights.len()];
    if old_weights.len() == weights.len() {
        for i in 0..old_weights.len() {
            if threshold < 0.0 {
                res[i] = weights[i].rounded() != old_weights[i].rounded();
            } else {
                res[i] = (weights[i].0 - old_weights[i].0).abs() >= threshold;
            }
        }
    }
    res
}

/// Like [`write_phased`], but each entry takes up at least `width` chars
#[must_use]
pub fn write_phased_with_width(
    weights: &[Weight],
    feature_idx: usize,
    special: &[bool],
    width: usize,
) -> String {
    let i = 2 * feature_idx;
    format!(
        "p({0}, {1})",
        weights[i].to_string(special.get(i).copied().unwrap_or_default(), width),
        weights[i + 1].to_string(special.get(i + 1).copied().unwrap_or_default(), width)
    )
}

/// Convert a pair of weights to string, coloring each one red if the corresponding `special` entry is set.
///
/// The two weight indices are `feature_idx * 2` and `feature_idx * 2 + 1`.
#[must_use]
pub fn write_phased(weights: &[Weight], feature_idx: usize, special: &[bool]) -> String {
    write_phased_with_width(weights, feature_idx, special, 0)
}

/// Returns a vector of [`Float`]s, where each entry counts to how often the corresponding feature appears in the
/// dataset, weighted by the sampling weight of its datapoint (which should usually be `1.0`).
/// This can be used to give a very rough idea of the variance of the weight.
#[must_use]
pub fn count_occurrences<D: Datapoint>(batch: Batch<D>) -> Vec<Float> {
    let mut res = vec![0.0; batch.num_weights];
    for datapoint in batch.iter() {
        for feature in datapoint.features() {
            res[feature.idx] += feature.weight.abs() * datapoint.sampling_weight();
        }
    }
    res
}

/// A post-processing step that interpolates the tuned weights with initial weights based on how often they appeared.
///
/// The `occurrences` slice counts how often a feature occurred in the dataset, see [`count_occurrences`].
/// If the [`interpolate_decay`][WeightsInterpretation::interpolate_decay] method of `interpretation` returns
/// [`Some`] `decay` value, the interpolation factor is computed as `decay^occurrence` (where `^` denotes the power operator):
///
/// If a weight was initially `0`, got tuned to `100` based on `2` samples, and the `decay` value is `0.9`, the resulting
/// weight is `0.9 * 0.9 * 0 + (1.0 - 0.9 * 0.9) * 100 == 19`.
///
/// This function should rarely be necessary. It is intended as a workaround for insufficient datasets, but the better
/// option is generally to use a larger dataset. Initial weights should be initialized to sensible priors when using this
/// function.
pub fn interpolate(
    occurrences: &[Float],
    weights: &mut Weights,
    interpretation: &dyn WeightsInterpretation,
) {
    if let Some(decay) = interpretation.interpolate_decay() {
        assert!(
            (0.0..1.0).contains(&decay),
            "decay must be in [0, 1) -- if you want no decay, simply return `None` in `initial_weights`."
        );
        let initial_weights = interpretation
            .initial_weights()
            .expect("Initial weights are needed for interpolating");
        assert_eq!(
            initial_weights.num_weights(),
            weights.num_weights(),
            "weights don't match up with initial weights"
        );
        assert_eq!(
            occurrences.len(),
            weights.num_weights(),
            "occurrences appear to have been generated for different weights"
        );
        for (idx, weight) in weights.iter_mut().enumerate() {
            let factor = decay.powf(occurrences[idx]);
            assert!((0.0..=1.0).contains(&factor), "internal error");
            *weight = initial_weights[idx] * factor + *weight * (1.0 - factor);
        }
    }
}

/// Returned by the [`eval_scale`](WeightsInterpretation::eval_scale) method.
///
/// The `InitialWeights` variant is useful when importing an existing eval, but `Scale` should generally be preferred
/// because it is less dependent on the tuning dataset and doesn't drift over time as new eval features are added.
pub enum EvalScale {
    /// Wraps a [`ScalingFactor`]
    Scale(ScalingFactor),
    /// Automatically tne the scaling factor based by minimizing the loss of the initial weights on a given batch.
    InitialWeights(Weights),
}

impl EvalScale {
    /// Converts this object into a [`ScalingFactor`].
    ///
    /// This is a no-op for the `Scale` variant, and uses gradient-based binary search for the `InitialWeights` variant.
    /// This is done on a best-effort basis. If the eval frequently mistakes who's winning and if many outcomes are near 0.5,
    /// this will try to minimize the loss by tuning the scale to infinity, thereby making the eval predict a draw in all cases.
    /// So don't blindly trust the result of this function, and try using WDL outcomes instead of eval outcomes if the scale
    /// gets turned to infinity. It's generally better to use a fixed scaling factor, tuning the scaling factor based on the
    /// initial weights is mostly used to import weights that haven't been tuned with this tuner;
    /// as soon as it has been used and resulted in a satisfactory eval scale, you should use that through the `Scale` variant.
    pub fn to_scaling_factor<B: Board, D: Datapoint, E: Eval<B>>(
        self,
        batch: Batch<D>,
        eval: &E,
    ) -> ScalingFactor {
        match self {
            Scale(scale) => scale,
            InitialWeights(weights) => tune_scaling_factor(&weights, batch, eval),
        }
    }
}

/// This function returns an object which implements [`Display`] by calling [`display`] on the [`WeightsInterpretation`]
/// with the supplied [`Weights`].
pub fn display<'a, E: WeightsInterpretation + ?Sized>(
    this: &'a E,
    weights: &'a Weights,
    old_weights: &'a [Weight],
) -> FormatWeights<'a> {
    FormatWeights {
        format_weights: this.display(),
        weights,
        old_weights,
    }
}

/// This trait deals with how your eval interprets weights: You only need to implement the [`display`](Self::display) method to
/// display the tuned weights.
///
/// The other methods are optional and can be implemented to change the default behavior.
/// If you want to import an existing eval, you should implement the [`initial_weights`](Self::initial_weights) method. Then, you can
/// implement the [`eval_scale`](Self::eval_scale) method to automatically adjust the scale of the tuned values so that they match your
/// existing values. Implementing [`initial_weights`](Self::initial_weights) also unlocks setting [`retune_from_zero`](Self::retune_from_zero) to `false`, which will
/// cause the tuner to start from your existing values. This can help it converge faster and to better results.
/// The [`interpolate_decay`](Self::interpolate_decay) method is rarely needed, its purpose is to interpolate between tuned weights and initial weights
/// based on how many samples for a given weight were in the training dataset. This can improve results if some weights
/// occur very rarely in the dataset, although the best course of action would still be to remedy this deficiency in the dataset.
pub trait WeightsInterpretation {
    /// This function should not be called directly, but it is the only methods that is required to be implemented.
    ///
    /// Its purpose is to return a function that prints the weights in a format that can be used by the actual engine.
    /// The `old_weights` parameter can safely be ignored, its purpose is to highlight changes in weights in a
    /// human-readably way, such as by coloring weights with large changes red. The [`changed_at_least`] function
    /// can be called to help implement this.
    fn display(
        &self,
    ) -> fn(f: &mut Formatter, weights: &Weights, old_weights: &[Weight]) -> std::fmt::Result;

    /// The eval scale is used to convert a [centipawn score](  gd::CpScore) in `(-∞, ∞)` to a winrate prediction
    /// in `(-1, 1)`.
    ///
    /// For chess, a Scale of `100` corresponds *very* roughly to a pawn value of 100 centipawns.
    /// Many engines are sensitive to scaling the eval by a linear factor like this, so the option to tune it
    /// automatically based on existing weights is meant to be used for importing an existing eval.
    /// See below for a more in-depth explanation; in general, explicitly setting the scale is preferred as that is
    /// more robust to e.g. changes in the dataset and doesn't drift over time.
    fn eval_scale(&self) -> EvalScale {
        if let Some(weights) = self.initial_weights() {
            InitialWeights(weights)
        } else {
            Scale(110.0) // gives roughly the normal piece values, expressed as centipawns, for chess
        }
    }

    /// Determines if weights are initialized to zero.
    ///
    /// If this returns [`false`], then the tuned weights are initialized to the return value of [`initial_weights`](Self::initial_weights).
    /// If that function returns [`None`], the program panics.
    /// If this function returns [`true`], weights are initialized to zero, which makes it easier to debug the eval and to
    /// get consistent results that don't hide problems with the eval, like the gradient of a weight being always zero,
    /// which happens if a feature doesn't appear in the dataset -- in this case, the initial weight will remain unchanged.
    fn retune_from_zero(&self) -> bool {
        true
    }

    /// Optional interpolation between tuned and initial weights, rarely needed.
    ///
    /// If this is `Some(decay)`, [`initial_weights`](Self::initial_weights) must return `Some(initial)`.
    /// This function then interpolates the `initial` weights with the tuned
    /// weights, where the interpolation factor of an initial weight is the `decay` value raised to the `n`th power, where `n` is,
    /// roughly speaking, how often the feature occurs. More concretely, `n` is the sum of the weight times the absolute value
    /// of the feature, summed over all positions. This is useful for getting prior knowledge into the tuned values in cases
    /// where features very rarely appear in the dataset, e.g. a king on the opponent's backrank in the middle game.
    fn interpolate_decay(&self) -> Option<Float> {
        None
    }

    /// Optionally returns preexisting weights.
    ///
    /// This function can be used to compute an eval scaling factor based on
    /// those weights. Note that the result can heavily depend on the datasets used and can fail for an eval that frequently
    /// miss-predicts who is winning, see [`EvalScale::to_scaling_factor`] for a more in-depth explanation.
    /// It can also be used as a starting point to retune weights from, which can drastically speed up the tuning process,
    /// but can also influence the results (which can often be a good thing), if gradients are essentially zero.
    fn initial_weights(&self) -> Option<Weights> {
        None
    }
}

/// Using this tuner means implementing this trait.
///
/// This first means deriving the [`WeightsInterpretation`] trait, an object safe base trait that does not
/// depend on the [`Board`] type. Implementing the `Eval` requires specifying the
/// number of weights and features, the type of a single [`Datapoint`] (e.g. [`NonTaperedDatapoint`](gd::NonTaperedDatapoint)),
/// a [`Filter`] that gets called when loading FENs (e.g. [`NoFilter`](super::load_data::NoFilter)), and implementing the [`feature_trace`][Self::feature_trace] method.
pub trait Eval<B: Board>: WeightsInterpretation + Default {
    /// For a normal, non-tapered, eval, the number of weights is the same as the number of features.
    /// For a tapered eval, it's twice the number of features. It would also be perfectly fine, if unusual,
    /// to only taper some features or to use 3 phases.
    /// Conceptually, this should be a compile time constant. However, Rust's compile time computation is so limited that it's
    /// sometimes more convenient to calculate this at runtime.
    fn num_weights() -> usize;

    /// A feature is a property of the position that gets recognized by the eval.
    ///
    /// For example, for a piece square table only eval, the number of features is the number
    /// of squares times the number of pieces. Each feature value counts how often the feature appears in a position,
    /// from white's perspective (so the equivalent black feature count gets subtracted).
    /// See also [`feature_trace`](Self::feature_trace).
    /// Conceptually, this should be a compile time constant. However, Rust's compile time computation is so limited that it's
    /// sometimes more convenient to calculate this at runtime.
    // TODO: Remove this requirement?
    fn num_features() -> usize;

    /// How a position is represented in the tuner.
    ///
    /// [`NonTaperedDatapoint`](gd::NonTaperedDatapoint) should be the default choice,
    /// [`TaperedDatapoint`](gd::TaperedDatapoint) is obviously useful for a tapered eval, and [`WeightedDatapoint`](gd::WeightedDatapoint) is the combination of a
    /// [`TaperedDatapoint`](gd::TaperedDatapoint) and a sampling weight; this is rarely necessary.
    type D: Datapoint;

    /// The [`Filter`] gets applied when loading a position.
    ///
    /// It can be used to e.g. remove noisy positions, such as
    /// when a king is in check in chess, or to perform a quiescent search on them to quieten them down, to change the
    /// outcome of a position by interpolating with the eval of an engine, to expand a single position into several
    /// related positions, etc.
    type Filter: Filter<B>;

    /// Converts a position into a datapoint.
    ///
    /// This method gets called when loading a dataset; its purpose is to extracts a list of features from the position.
    /// These features get then turned into weights, which are tuned automatically.
    /// Although it is possible to implement this method directly, the recommended route is to implement
    /// [`feature_trace`](Self::feature_trace) instead.
    fn extract_features(pos: &B, outcome: Outcome, weight: Float) -> Self::D {
        Self::D::new(Self::feature_trace(pos), outcome, weight)
    }

    /// Converts a position into a [trace](TraceTrait).
    ///
    /// The advantage of implementing this method over [`extract_features`](Eval::extract_features) is that it's often much more convenient to
    /// calculate the trace, which then gets turned into a [`Datapoint`] in a separate step. A trace lists how often a
    /// feature appears for each player, so unlike a Datapoint, it still contains semantic information instead of a
    /// single one-dimensional list of feature counts.
    /// [`TuneLiTEval`] is an example for how an existing eval function can be used to create a feature trace without
    /// needing to duplicate the eval implementation.
    fn feature_trace(pos: &B) -> impl TraceTrait;
}

/// Implementation detail of the eval formatting.
pub struct FormatWeights<'a> {
    format_weights: fn(&mut Formatter<'_>, &Weights, &[Weight]) -> std::fmt::Result,
    weights: &'a Weights,
    old_weights: &'a [Weight],
}

impl<'a> Display for FormatWeights<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        (self.format_weights)(f, self.weights, self.old_weights)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Display)]
enum Direction {
    Up,
    Down,
}

fn grad_for_eval_scale<D: Datapoint>(
    weights: &Weights,
    batch: Batch<D>,
    eval_scale: ScalingFactor,
) -> (Direction, Float) {
    // the gradient of the loss function with respect to 1/eval_scale, ignoring constant factors
    let mut scaled_grad = 0.0;
    let mut loss = 0.0;
    for data in batch.datapoints {
        let cp_eval = cp_eval_for_weights(weights, data);
        let prediction = cp_to_wr(cp_eval, eval_scale);
        let outcome = data.outcome();
        let sample_grad = <DefaultOptimizer as Optimizer<D>>::Loss::sample_gradient(
            prediction,
            outcome,
            data.sampling_weight(),
        ) * cp_eval.0;
        scaled_grad += sample_grad;
        loss += sample_loss(prediction, data.outcome()) * data.sampling_weight();
    }
    loss /= batch.weight_sum as Float;
    // the gradient tells us how we need to change 1/eval_scale to maximize the loss, which is the same direction
    // as changing eval_scale to minimize the loss.
    let dir = if scaled_grad > 0.0 { Up } else { Down };
    println!("Current eval scale {eval_scale:.2}, loss {loss}, direction: {dir}");
    (dir, loss)
}

fn tune_scaling_factor<B: Board, D: Datapoint, E: Eval<B>>(
    weights: &Weights,
    batch: Batch<D>,
    eval: &E,
) -> ScalingFactor {
    assert_eq!(
        E::num_weights(),
        weights.len(),
        "The batch doesn't seem to have been created by this eval function"
    );
    assert_eq!(
        weights.len(),
        batch.num_weights,
        "Incorrect number of weights: The eval claims to have {0} weights, but the weights used for tuning the scaling factor have {1} entries",
        batch.num_weights,
        weights.len()
    );
    let mut scale = 100.0;
    let mut prev_dir = None;
    assert!(
        !weights.iter().all(|w| w.0 == 0.0),
        "All weights are zero; can't tune a scaling factor. This may be due to a bugged eval or empty dataset"
    );
    println!(
        "Optimizing scaling factor for eval:\n{}",
        display(eval, weights, &[])
    );
    // First, do exponential search to find an interval in which we know that the optimal value lies.
    loop {
        assert!(!(scale >= 1e9 || scale <= 1e-9),
            "The eval scale doesn't seem to converge. This may be due to a bugged eval implementation or simply \
            because the eval fails to accurately predict the used datasets. You can always fall back to hand-picking an \
            eval scale in case this doesn't work, or try again with different datasets");
        let (dir, _loss) = grad_for_eval_scale(weights, batch, scale);
        if prev_dir.is_none() {
            prev_dir = Some(dir);
        } else if prev_dir.unwrap() != dir {
            break;
        }
        match dir {
            Up => scale *= 2.0,
            Down => scale /= 2.0,
        }
    }
    // Then, do binary search.
    let (mut lower_bound, mut upper_bound) = match prev_dir.unwrap() {
        Up => (scale / 2.0, scale),
        Down => (scale, scale * 2.0),
    };
    let loss_threshold = 0.01;
    loop {
        scale = (upper_bound + lower_bound) / 2.0;
        let (dir, loss) = grad_for_eval_scale(weights, batch, scale);
        if loss < loss_threshold || upper_bound - scale <= 0.01 {
            return scale;
        }
        match dir {
            Up => lower_bound = scale,
            Down => upper_bound = scale,
        }
    }
}
