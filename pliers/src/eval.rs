use crate::gd::{Position, Weights};
use gears::games::Board;
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;

pub mod caps_hce_eval;

#[derive(Default)]
pub struct FormatWeights<B: Board, E: Eval<B>> {
    weights: Weights,
    _phantom_data1: PhantomData<B>,
    _phantom_data2: PhantomData<E>,
}

impl<B: Board, E: Eval<B>> FormatWeights<B, E> {
    pub fn new(weights: Weights) -> Self {
        Self {
            weights,
            ..Default::default()
        }
    }
}

impl<B: Board, E: Eval<B>> Display for FormatWeights<B, E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        assert_eq!(self.weights.len(), E::NUM_FEATURES);
        E::format_impl(f, &self.weights)
    }
}

pub trait Eval<B: Board>: Default {
    const NUM_FEATURES: usize;
    fn features(pos: &B) -> Position;

    fn format_impl(f: &mut Formatter<'_>, weights: &Weights) -> std::fmt::Result;

    fn formatter(weights: Weights) -> FormatWeights<B, Self>
    where
        Self: Sized,
    {
        FormatWeights::new(weights)
    }
}
