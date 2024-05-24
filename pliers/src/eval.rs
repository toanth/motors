use crate::gd::{Datapoint, Outcome, TraceTrait, Weights};
use crate::load_data::{Filter, NoFilter};
use gears::games::Board;
use gears::general::bitboards::RawBitboard;
use std::fmt::{Display, Formatter};

pub mod chess;

pub struct FormatWeights<'a> {
    format_weights: fn(&mut Formatter<'_>, &Weights) -> std::fmt::Result,
    weights: &'a Weights,
}

impl<'a> Display for FormatWeights<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        (self.format_weights)(f, self.weights)
    }
}

pub trait WeightFormatter {
    fn formatter<'a>(&'a self, weights: &'a Weights) -> FormatWeights {
        FormatWeights {
            format_weights: self.format_impl(),
            weights,
        }
    }

    fn format_impl(&self) -> (fn(f: &mut Formatter, weights: &Weights) -> std::fmt::Result);
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
}
