use crate::gd::{Float, Position, Weights};
use gears::games::Board;
use gears::general::bitboards::RawBitboard;
use std::fmt::{Display, Formatter};

pub mod chess;

pub struct FormatWeights {
    weights: Option<Weights>,
    format_fn: fn(&mut Formatter<'_>, &Weights) -> std::fmt::Result,
    num_features: usize,
}

impl FormatWeights {
    pub fn new<B: Board, E: Eval<B>>() -> Self {
        Self {
            weights: None,
            format_fn: E::format_impl,
            num_features: E::NUM_FEATURES,
        }
    }
    pub fn with_weights(&mut self, weights: Weights) -> &Self {
        self.weights = Some(weights);
        self
    }
}

impl Display for FormatWeights {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        assert_eq!(self.weights.as_ref().unwrap().len(), self.num_features);
        (self.format_fn)(f, self.weights.as_ref().unwrap())
    }
}

pub trait Eval<B: Board>: Default {
    const NUM_FEATURES: usize;
    fn features(pos: &B) -> Position;

    fn format_impl(f: &mut Formatter<'_>, weights: &Weights) -> std::fmt::Result;

    fn formatter() -> FormatWeights
    where
        Self: Sized,
    {
        FormatWeights::new::<B, Self>()
    }
}
