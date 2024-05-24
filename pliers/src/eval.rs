use crate::eval::Direction::{Down, Up};
use crate::eval::EvalScale::{InitialWeights, Scale};
use crate::gd::{
    sample_loss, wr_prediction_for_weights, Batch, Datapoint, Float, Outcome, ScalingFactor,
    TraceTrait, Weights,
};
use crate::load_data::{Filter, NoFilter};
use gears::games::Board;
use gears::general::bitboards::RawBitboard;
use std::fmt::{Display, Formatter};

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
    pub fn to_scaling_factor<D: Datapoint>(self, batch: Batch<D>) -> ScalingFactor {
        match self {
            Scale(scale) => scale,
            InitialWeights(weights) => tune_scaling_factor(weights, batch),
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

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Direction {
    Up,
    Down,
}

fn grad_for_eval_scale<D: Datapoint>(
    weights: &Weights,
    batch: Batch<D>,
    eval_scale: ScalingFactor,
) -> (Direction, Float) {
    let mut scaled_grad = 0.0;
    let mut loss = 0.0;
    for data in batch.datapoints {
        let prediction = wr_prediction_for_weights(&weights, data, eval_scale);
        scaled_grad += (prediction.0 - data.outcome().0) * prediction.0 * (1.0 - prediction.0);
        loss += sample_loss(prediction, data.outcome());
    }
    loss /= batch.datapoints.len() as Float;
    let dir = if scaled_grad > 0.0 { Down } else { Up };
    (dir, loss)
}

/// Takes in an initial set of weights and tunes the eval scale to minimize the loss of those weights on the batch
fn tune_scaling_factor<D: Datapoint>(weights: Weights, batch: Batch<D>) -> ScalingFactor {
    let mut scale = 100.0;
    let loss_threshold = 0.01;
    let mut prev_dir = None;
    // First, do exponential search to find an interval in which we know that the optimal value lies.
    loop {
        let (dir, loss) = grad_for_eval_scale(&weights, batch, scale);
        if loss < loss_threshold {
            break;
        }
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
    loop {
        scale = (upper_bound + lower_bound) / 2.0;
        let (dir, loss) = grad_for_eval_scale(&weights, batch, scale);
        if loss < loss_threshold || upper_bound - loss <= 0.1 {
            return scale;
        }
        match dir {
            Up => lower_bound = scale,
            Down => upper_bound = scale,
        }
    }
}
