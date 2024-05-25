use crate::eval::Direction::{Down, Up};
use crate::eval::EvalScale::{InitialWeights, Scale};
use crate::gd::{
    cp_eval_for_weights, cp_to_wr, sample_loss, wr_prediction_for_weights, Batch, Datapoint, Float,
    Outcome, ScalingFactor, TraceTrait, Weights,
};
use crate::load_data::{Filter, NoFilter};
use derive_more::Display;
use gears::games::Board;
use gears::general::bitboards::RawBitboard;
use std::fmt::Formatter;

pub mod chess;

pub trait WeightFormatter {
    fn formatter<'a>(&'a self, weights: &'a Weights) -> FormatWeights {
        FormatWeights {
            format_weights: self.format_impl(),
            weights,
        }
    }

    fn format_impl(&self) -> (fn(f: &mut Formatter, weights: &Weights) -> std::fmt::Result);
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

pub trait Eval<B: Board>: WeightFormatter + Default {
    const NUM_WEIGHTS: usize;
    const NUM_FEATURES: usize;

    type D: Datapoint;

    type Filter: Filter<B>;

    fn extract_features(pos: &B, outcome: Outcome) -> Self::D {
        Self::D::new(Self::feature_trace(pos), outcome)
    }

    fn feature_trace(pos: &B) -> impl TraceTrait;

    fn eval_scale() -> EvalScale {
        if let Some(weights) = Self::initial_weights() {
            InitialWeights(weights)
        } else {
            Scale(110.0) // gives roughly the normal piece values, expressed as centipawns, for chess
        }
    }

    /// When using this tuner for existing weights, this function can be used to compute an eval scaling factor based on
    /// those weights. Note that the result can heavily depend on the datasets used and can fail for an eval that frequently
    /// misspredicts who's winning, see `tune_scaling_factor` below for a more in-depth explanation.
    fn initial_weights() -> Option<Weights> {
        None
    }
}

/// Here follow implementation details of the eval.

pub struct FormatWeights<'a> {
    format_weights: fn(&mut Formatter<'_>, &Weights) -> std::fmt::Result,
    weights: &'a Weights,
}

impl<'a> Display for FormatWeights<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        (self.format_weights)(f, self.weights)
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
        let cp_eval = cp_eval_for_weights(&weights, data);
        let prediction = cp_to_wr(cp_eval, eval_scale);
        let outcome = data.outcome().0;
        let sample_grad =
            (prediction.0 - outcome) * prediction.0 * (1.0 - prediction.0) * cp_eval.0;
        scaled_grad += sample_grad;
        loss += sample_loss(prediction, data.outcome());
    }
    loss /= batch.datapoints.len() as Float;
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
        eval.formatter(&weights)
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
