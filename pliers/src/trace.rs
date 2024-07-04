/*
 *  Pliers, a tuner for engine evaluation weights.
 *  Copyright (C) 2024 ToTheAnd
 *
 *  Pliers is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Pliers is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Pliers. If not, see <https://www.gnu.org/licenses/>.
 */

//! A trace is used in the eval function to gather information about the position.
//! It is then converted to a list of features.
//!
//! Each eval function should define its custom trace. There are two main ways to do this:
//! The easiest way is to build a custom trace on top of nested traces,
//! usually of type [`SimpleTrace`], [`TraceNFeatures`] or [`SparseTrace`], and then write a custom implementation
//! of the [`Eval::feature_trace`] method.
//! Alternatively, the type [`SparseTrace`] implements the [`ScoreType`] trait, so it can be substituted for the score
//! in an existing eval function. This avoids having to repeat the eval function implementation for the tuner.
//! [`TuneLiTEval`] is an example of how such an implementation can look like.

use crate::gd::{Feature, FeatureT, Float};
use gears::games::Color;
use gears::score::PhaseType;
use motors::eval::ScoreType;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

// TODO: Only a single generic trace type

type FeatureIndex = usize;

type FeatureCount = isize;

/// A single feature, building block of [`SparseTrace`].
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq)]
pub struct SingleFeature {
    idx: FeatureIndex,
    count: FeatureCount,
}

impl SingleFeature {
    pub(super) fn new(idx: FeatureIndex) -> Self {
        Self { idx, count: 1 }
    }

    pub(super) fn no_feature() -> Self {
        Self::default()
    }
}

impl Mul<usize> for SingleFeature {
    type Output = Self;

    fn mul(mut self, rhs: usize) -> Self::Output {
        let rhs: FeatureCount = rhs.try_into().unwrap();
        self.count = self.count * rhs;
        self
    }
}

/// A trace that stores a map from feature index to feature count.
///
/// Implements `ScoreType` so that it can be used instead of a normal score for an existing eval.
/// This makes it possible to write one eval function that serves both as a normal eval function for
/// an engine and to generate a trace for tuning.
#[derive(Debug, Default, Clone)]
pub struct SparseTrace {
    map: HashMap<FeatureIndex, FeatureCount>,
    phase: Float,
}

impl From<SingleFeature> for SparseTrace {
    fn from(value: SingleFeature) -> Self {
        Self {
            map: HashMap::from([(value.idx, value.count)]),
            phase: 0.0,
        }
    }
}

impl SparseTrace {
    fn merge(&mut self, other: SparseTrace, negate_other: bool) {
        for (key, val) in other.map.iter() {
            let val = match negate_other {
                true => -*val,
                false => *val,
            };
            match self.map.entry(*key) {
                Entry::Occupied(o) => {
                    *o.into_mut() += val;
                }
                Entry::Vacant(v) => {
                    v.insert(val);
                }
            }
        }
    }
}

impl TraceTrait for SparseTrace {
    fn as_features(&self, idx_offset: usize) -> Vec<Feature> {
        let mut res = vec![];
        for (index, feature) in self.map.iter() {
            let count: FeatureT = (*feature).try_into().unwrap();
            if count != 0 {
                res.push(Feature::new(
                    (*feature).try_into().unwrap(),
                    (index + idx_offset).try_into().unwrap(),
                ))
            }
        }
        res.sort_by_key(|f| f.idx());
        res
    }

    fn nested_traces(&self) -> Vec<&dyn TraceTrait> {
        vec![]
    }

    fn phase(&self) -> Float {
        self.phase
    }

    fn max_num_features(&self) -> usize {
        todo!()
    }
}

impl Add for SparseTrace {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign for SparseTrace {
    fn add_assign(&mut self, rhs: Self) {
        self.merge(rhs, false);
    }
}

impl Add<SingleFeature> for SparseTrace {
    type Output = Self;

    fn add(mut self, rhs: SingleFeature) -> Self::Output {
        self += rhs;
        self
    }
}

impl AddAssign<SingleFeature> for SparseTrace {
    fn add_assign(&mut self, rhs: SingleFeature) {
        let entry = self.map.entry(rhs.idx);
        *entry.or_default() += rhs.count;
    }
}

impl Sub for SparseTrace {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self::Output {
        self -= rhs;
        self
    }
}

impl SubAssign for SparseTrace {
    fn sub_assign(&mut self, rhs: Self) {
        self.merge(rhs, true);
    }
}

impl Sub<SingleFeature> for SparseTrace {
    type Output = Self;

    fn sub(mut self, rhs: SingleFeature) -> Self::Output {
        self -= rhs;
        self
    }
}

impl SubAssign<SingleFeature> for SparseTrace {
    fn sub_assign(&mut self, rhs: SingleFeature) {
        let entry = self.map.entry(rhs.idx);
        *entry.or_default() -= rhs.count;
    }
}

impl Neg for SparseTrace {
    type Output = Self;

    fn neg(mut self) -> Self::Output {
        for count in self.map.values_mut() {
            *count = -*count;
        }
        self
    }
}

impl Mul<usize> for SparseTrace {
    type Output = Self;

    fn mul(mut self, rhs: usize) -> Self::Output {
        for count in self.map.values_mut() {
            *count = (*count as usize * rhs).try_into().unwrap();
        }
        self
    }
}

impl PartialEq for SparseTrace {
    fn eq(&self, other: &Self) -> bool {
        self.map.eq(&other.map)
    }
}

impl Eq for SparseTrace {}

impl ScoreType for SparseTrace {
    type Finalized = Self;
    type SingleFeatureScore = SingleFeature;

    fn finalize(
        mut self,
        phase: PhaseType,
        max_phase: PhaseType,
        _color: Color,
        _tempo: Self::Finalized,
    ) -> Self::Finalized {
        self.phase = phase as Float / max_phase as Float;
        self
    }
}

/// A trace stores extracted features of a position and can be converted to a list of [`Feature`]s.
///
/// This type is returned by the [`feature_trace`](super::eval::Eval::feature_trace) method.
/// The simplest way to implement this trait is to make your strut contain several [`SimpleTrace`]s or other
/// pre-defined trace implementation, which do the actual work of converting the trace to a list of features.
///
/// For example:
/// ```
/// use pliers::gd::{Feature, Float};
/// use pliers::trace::{SimpleTrace, TraceNFeatures, TraceTrait};
/// #[derive(Debug, Default)]
/// struct MyTrace {
///     some_trace: SimpleTrace,
///     some_other_trace: TraceNFeatures<42>,
/// }
///
/// impl TraceTrait for MyTrace {
///     fn nested_traces(&self) -> Vec<&dyn TraceTrait> {
///         vec![&self.some_trace, &self.some_other_trace]
///    }
///
///     fn phase(&self) -> Float {
///        1.0
///    }
/// }
/// ```
pub trait TraceTrait: Debug {
    /// Converts the trace into a list of features.
    ///
    /// The default implementation of this function simply delegates the work to nested traces.
    /// It is usually not necessary to override this default implementation.
    /// This function creates a sparse array of [`Feature`]s, where each entry is the number of times it appears for
    /// the white player minus the number of times it appears for the black player.
    fn as_features(&self, idx_offset: usize) -> Vec<Feature> {
        let mut res = vec![];
        let mut offset = idx_offset;
        for nested in self.nested_traces() {
            res.append(&mut nested.as_features(offset));
            offset += nested.max_num_features();
        }
        res
    }

    /// Returns an iterator of nested traces.
    ///
    /// A custom trace should be built on top of existing traces, such as [`TraceNFeatures`].
    /// The order of traces in the returned `Vec` determines the offset used to convert the feature index of a single
    /// trace into the feature index of the merged trace.
    fn nested_traces(&self) -> Vec<&dyn TraceTrait>;

    // TODO: Remove this from the trait
    /// The phase value of this position. Some [`Datapoint`] implementations ignore this.
    fn phase(&self) -> Float;

    /// The number of features that are being covered by this trace.
    ///
    /// Note that in many cases, not all features appear in a position, so the len of the result of
    /// [`as_features`](Self::as_features) is often smaller than this value.
    /// It is usually not necessary to override this method.
    fn max_num_features(&self) -> usize {
        self.nested_traces()
            .iter()
            .map(|trace| trace.max_num_features())
            .sum()
    }
}

/// A trace that keeps track of a given feature, which is referred to by its index.
///
/// Can be used to build larger traces. It is usually not necessary to implement this trait yourself
/// because [`SimpleTrace`] and [`TraceNFeatures`] already do.
pub trait BasicTrace: TraceTrait {
    /// Increment a given feature by one for the given player.
    fn increment(&mut self, idx: usize, color: Color) {
        self.increment_by(idx, color, 1);
    }

    /// Increment a given feature by a given amount for the given player.
    fn increment_by(&mut self, idx: usize, color: Color, amount: isize);
}

/// The most basic trace, useful by itself or as a building block of custom traces, but [`TraceNFeatures`]
/// should usually be preferred.
///
/// Stores how often each feature occurs for both players, and a game phase.
/// Unlike the final list of `Feature`s used during tuning, this uses a dense array representation,
/// which means it is normal for most of the many entries to be zero.
/// The [`TraceNFeatures]` struct is a thin wrapper around this struct which enforces the number of features matches.
#[derive(Debug, Default, Clone)]
pub struct SimpleTrace {
    /// How often each feature appears for the white player.
    pub white: Vec<isize>,
    /// How often each feature appears for the black player.
    pub black: Vec<isize>,
    /// The phase value. Only needed for tapered evaluations.
    pub phase: Float,
}

impl SimpleTrace {
    /// Create a trace of `num_feature` elements, all initialized to zero.
    /// Also sets the `phase` to zero.
    pub fn for_features(num_features: usize) -> Self {
        Self {
            white: vec![0; num_features],
            black: vec![0; num_features],
            phase: 0.0,
        }
    }
}

impl TraceTrait for SimpleTrace {
    /// A [`SimpleTrace`] does not contain any other traces, so this function does the actual work of converting
    /// a trace into a list of features.
    fn as_features(&self, idx_offset: usize) -> Vec<Feature> {
        assert_eq!(self.white.len(), self.black.len());
        let mut res = vec![];
        for i in 0..self.white.len() {
            let diff = self.white[i] - self.black[i];
            if diff != 0 {
                let idx = i + idx_offset;
                assert!(diff >= FeatureT::MIN as isize && diff <= FeatureT::MAX as isize);
                assert!(res.len() < u16::MAX as usize);
                assert!(idx <= u16::MAX as usize);
                let feature = Feature::new(diff as FeatureT, idx as u16);
                res.push(feature);
            }
        }
        res.sort_by_key(|a| a.idx());
        res
    }

    fn nested_traces(&self) -> Vec<&dyn TraceTrait> {
        vec![]
    }

    fn phase(&self) -> Float {
        self.phase
    }

    fn max_num_features(&self) -> usize {
        assert_eq!(self.black.len(), self.white.len());
        self.white.len()
    }
}

impl BasicTrace for SimpleTrace {
    fn increment_by(&mut self, idx: usize, color: Color, amount: isize) {
        match color {
            Color::White => self.white[idx] += amount,
            Color::Black => self.black[idx] += amount,
        };
    }
}

/// Wraps a [`SimpleTrace`] by making sure it has the given maximum number of features.
#[derive(Debug)]
pub struct TraceNFeatures<const N: usize>(pub SimpleTrace);

impl<const N: usize> Default for TraceNFeatures<N> {
    fn default() -> Self {
        Self(SimpleTrace::for_features(N))
    }
}

impl<const N: usize> TraceTrait for TraceNFeatures<N> {
    fn as_features(&self, idx_offset: usize) -> Vec<Feature> {
        assert_eq!(self.0.max_num_features(), N);
        self.0.as_features(idx_offset)
    }

    fn nested_traces(&self) -> Vec<&dyn TraceTrait> {
        self.0.nested_traces()
    }

    fn phase(&self) -> Float {
        self.0.phase
    }
    fn max_num_features(&self) -> usize {
        N
    }
}

impl<const N: usize> BasicTrace for TraceNFeatures<N> {
    fn increment_by(&mut self, idx: usize, color: Color, amount: isize) {
        self.0.increment_by(idx, color, amount);
    }
}

/// Trace for a single feature that can appear multiple times for both players.
pub type SingleFeatureTrace = TraceNFeatures<1>;
