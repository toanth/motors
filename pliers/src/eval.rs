use crate::eval::Direction::{Down, Up};
use crate::eval::EvalScale::{InitialWeights, Scale};
use crate::gd::{
    cp_eval_for_weights, cp_to_wr, sample_loss, scaled_sample_grad, Batch, Datapoint, Float,
    Outcome, ScalingFactor, TraceTrait, Weight, Weights,
};
use crate::load_data::Filter;
use derive_more::Display;
use gears::games::Board;
use std::fmt::Formatter;

pub mod chess;

pub fn changed_at_least(threshold: Float, weights: &Weights, old_weights: &[Weight]) -> Vec<bool> {
    let mut res = vec![false; weights.len()];
    if old_weights.len() == weights.len() {
        for i in 0..old_weights.len() {
            res[i] = (weights[i].0 - old_weights[i].0).abs() >= threshold;
        }
    }
    res
}

/// Returns a vector of `<Float`s, where each entry counts to how often the corresponding feature appears in the
/// dataset, weighted by the sampling weight of its datapoint (which should usually be 1.0).
/// This can be used to give a very rough idea of the variance of the weight.
pub fn count_occurrences<D: Datapoint>(batch: Batch<D>) -> Vec<Float> {
    let mut res = vec![0.0; batch.num_weights];
    for datapoint in batch.iter() {
        for feature in datapoint.features() {
            res[feature.idx] += feature.weight.abs() * datapoint.sampling_weight();
        }
    }
    res
}

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

pub enum EvalScale {
    Scale(ScalingFactor),
    InitialWeights(Weights),
}

impl EvalScale {
    pub fn to_scaling_factor<B: Board, D: Datapoint, E: Eval<B>>(
        self,
        batch: Batch<D>,
        eval: &E,
    ) -> ScalingFactor {
        match self {
            Scale(scale) => scale,
            InitialWeights(weights) => tune_scaling_factor(weights, batch, eval),
        }
    }
}

pub trait WeightsInterpretation {
    /// This function should not be implemented directly. Instead, implement `display_impl`.
    fn display<'a>(&'a self, weights: &'a Weights, old_weights: &'a [Weight]) -> FormatWeights {
        FormatWeights {
            format_weights: self.display_impl(),
            weights,
            old_weights,
        }
    }

    /// This function should not be called directly, but implemented by a `struct` that implements this trait.
    /// Its purpose is to return a function that prints the weights in a format that can be used by the actual engine.
    /// The `old_weights` parameter can safely be ignored, its purpose is to highlight changes in weights in a
    /// human-readably way, such as by coloring weights with large changes red. The `changed_at_least` function
    /// can be called to help implement this.
    fn display_impl(
        &self,
    ) -> fn(f: &mut Formatter, weights: &Weights, old_weights: &[Weight]) -> std::fmt::Result;

    /// The eval scale is used to convert a centipawn score (in (-infinity, infinity))to a winrate prediction
    /// (in [-1, 1]). For chess, a Scale of 100 corresponds *very* roughly to a pawn value of 100 centipawns.
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

    /// If this returns `false`, then the tuned weights are initialized to the return value of `initial_weights`.
    /// If that function returns `None`, the program panics.
    /// If this function returns `true`, weights are initialized to zero, which makes it easier to debug the eval and to
    /// get consistent results that don't hide problems with the eval, like the gradient of a weight being always zero,
    /// which happens if a feature doesn't appear in the dataset -- in this case, the initial weight will remain unchanged.
    fn retune_from_zero(&self) -> bool {
        true
    }

    /// If this is `Some(decay)`, `initial_weights` must return `Some(initial)`. Interpolate initial weights with the tuned
    /// weights, where the interpolation factor of an initial weight is the sum of the weight times the absolute value
    /// of the feature, summed over all positions. This is useful for getting prior knowledge into the tuned values in cases
    /// where features very rarely appear in the dataset, e.g. a king on the opponent's backrank in the middle game.
    fn interpolate_decay(&self) -> Option<Float> {
        None
    }

    /// When using this tuner for existing weights, this function can be used to compute an eval scaling factor based on
    /// those weights. Note that the result can heavily depend on the datasets used and can fail for an eval that frequently
    /// misspredicts who's winning, see `tune_scaling_factor` below for a more in-depth explanation.
    /// It can also be used as a starting point to retune weights from, which can drastically speed up the tuning process,
    /// but can also influence the results sometimes (which can be a good thing), if gradients are essentially zero.
    /// An example of this would be squares at the 8th rank in the middlegame king piece square table; this basically never
    /// happens, so the tuned weights don't influence the loss much and the gradient can vanish.
    /// Despite those features not appearing in many games, they can still influence search and have a noticeable elo impact.
    fn initial_weights(&self) -> Option<Weights> {
        None
    }
}

/// Using this tuner for means implementing this trait.
pub trait Eval<B: Board>: WeightsInterpretation + Default {
    /// For a normal, non-tapered, eval, the number of weights is the same as the number of features.
    /// For a tapered eval, it's twice the number of features. It would also be perfectly fine, if unusual,
    /// to only taper some features or to use 3 phases.
    const NUM_WEIGHTS: usize;

    /// A feature is a property of the position that gets recognized by the eval.
    /// For example, for a piece square table only eval, the number of features is the number
    /// of squares times the number of pieces. Each feature value counts how often the feature appears in a position,
    /// from white's perspective (so the equivalent black feature count gets subtracted). See also `feature_trace`.
    const NUM_FEATURES: usize;

    /// How a position is represented in the tuner. `NonTaperedDatapoint` should be the default choice,
    /// `TaperedDatapoint` is obviously useful for tapered eval, and `WeightedDatapoint` is the combination of a
    /// `TaperedDatapoint` and a sampling weight; this is rarely useful, see also its Documentation.
    type D: Datapoint;

    /// The `Filter` gets applied when loading a position. It can be used to e.g. remove noisy positions, such as
    /// when a king is in check in chess, or to perform a quiescent search on them to quieten them down, to change the
    /// outcome of a position by interpolating with the eval of an engine, to expand a single position into several
    /// related positions, etc.
    type Filter: Filter<B>;

    /// This method gets called when loading a dataset; it converts a position into a Datapoint (which mostly means
    /// a list of features). It can be implemented directly, but the recommended route is to implement `feature_trace` instead.
    fn extract_features(pos: &B, outcome: Outcome, weight: Float) -> Self::D {
        Self::D::new(Self::feature_trace(pos), outcome, weight)
    }

    /// The advantage of implementing this method over `extract_features` is that it's often much more convenient to
    /// calculate the trace, which then gets turned into a `Datapoint` in a separate step. A trace lists how often a
    /// feature appears for each player, so unlike a Datapoint, it still contains semantic information instead of a
    /// single one-dimensional list of feature counts.
    fn feature_trace(pos: &B) -> impl TraceTrait;
}

/// Here follow implementation details of the eval.

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
        let sample_grad =
            scaled_sample_grad(prediction, outcome, data.sampling_weight()) * cp_eval.0;
        scaled_grad += sample_grad;
        loss += sample_loss(prediction, data.outcome(), data.sampling_weight());
    }
    loss /= batch.weight_sum as Float;
    // the gradient tells us how we need to change 1/eval_scale to maximize the loss, which is the same direction
    // as changing eval_scale to minimize the loss.
    let dir = if scaled_grad > 0.0 { Up } else { Down };
    println!("Current eval scale {eval_scale:.2}, loss {loss}, direction: {dir}");
    (dir, loss)
}

/// Takes in an initial set of weights and tunes the eval scale to minimize the loss of those weights on the batch.
/// This is done on a best-effort basis. If the eval frequently mistakes who's winning and if many outcomes are near 0.5,
/// this will try to minimize the loss by tuning the scale to infinity, thereby making the eval predict a draw in all cases.
/// So don't blindly trust the result of this function, and try using WDL outcomes instead of eval outcomes if the scale
/// gets turned to infinity. It's generally better to use a fixed scaling factor, this function is mostly used to import
/// weights that aren't tuned with this tuner; as soon as it has been used and resulted in a satisfactory eval scale, you should use that.
fn tune_scaling_factor<B: Board, D: Datapoint, E: Eval<B>>(
    weights: Weights,
    batch: Batch<D>,
    eval: &E,
) -> ScalingFactor {
    assert_eq!(
        E::NUM_WEIGHTS,
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
        eval.display(&weights, &[])
    );
    // First, do exponential search to find an interval in which we know that the optimal value lies.
    loop {
        if scale >= 1e9 || scale <= 1e-9 {
            panic!("The eval scale doesn't seem to converge. This may be due to a bugged eval implementation or simply \
            because the eval fails to accurately predict the used datasets. You can always fall back to hand-picking an \
            eval scale in case this doesn't work, or try again with different datasets");
        }
        let (dir, _loss) = grad_for_eval_scale(&weights, batch, scale);
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
        let (dir, loss) = grad_for_eval_scale(&weights, batch, scale);
        if loss < loss_threshold || upper_bound - scale <= 0.01 {
            return scale;
        }
        match dir {
            Up => lower_bound = scale,
            Down => upper_bound = scale,
        }
    }
}
