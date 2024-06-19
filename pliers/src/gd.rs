//! Everything related to the actual optimization, using a Gradient Descent-based tuner ([`Adam`] by default).

use crate::eval::{count_occurrences, display, interpolate, WeightsInterpretation};
use colored::Colorize;
use derive_more::{Add, AddAssign, Deref, DerefMut, Display, Div, Mul, Sub, SubAssign};
use gears::games::Color;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use rayon::prelude::*;
use std::fmt::{Debug, Formatter};
use std::ops::{DivAssign, MulAssign};
use std::time::Instant;
use std::usize;

// TODO: Better value
/// If the batch size exceeds this value, a multithreaded implementation will be used for computing the gradient and loss.
/// Not doing multithreading for small batch sizes isn't only meant to improve performance,
/// it also makes it easier to debug problems with the eval because stack traces and debugger steps
/// are simpler.
pub const MIN_MULTITHREADING_BATCH_SIZE: usize = 10_000;

/// Gradient Descent based tuning works with real numbers. This is the type used to represent those.
pub type Float = f64;

/// The result of calling the eval function.
///
/// Although a real eval function usually uses integer weights and only produces integer results,
/// during tuning, weights are stored as [`Float`]s, which is why this type also wraps a [`Float`].
/// Tuning works by comparing the actual [`Outcome`] to the predicted [`WrScore`].
/// For this, the [`CpScore`] is converted into a [`WrScore`] by applying a [`sigmoid`].
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub struct CpScore(pub Float);

impl Display for CpScore {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}cp", self.0)
    }
}

/// The win rate prediction, based on the [`CpScore`] (between `0` and `1`).
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub struct WrScore(pub Float);

/// `WrScore` is used for the converted score returned by the eval, [`Outcome`] for the actual outcome.
pub type Outcome = WrScore;

impl WrScore {
    /// Construct a new [`WrScore`] from a [`Float`].
    /// panics if the [`Float`] is not within the interval `[-1, 1]`.
    pub fn new(val: Float) -> Self {
        assert!((0.0..=1.0).contains(&val));
        Self(val)
    }
}

impl Display for WrScore {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

/// The eval scale stretches the [`sigmoid`] horizontally, so a larger eval scale means that a larger eval value
/// is necessary to count as "surely lost/won". It determines how to convert a [`CpScore`] to a [`WrScore`].
pub type ScalingFactor = Float;

/// [Logistic sigmoid](<https://en.wikipedia.org/wiki/Logistic_function#Mathematical_properties>),
/// dividing `x` by a [`ScalingFactor`].
pub fn sigmoid(x: Float, scale: ScalingFactor) -> Float {
    1.0 / (1.0 + (-x / scale).exp())
}

/// Convert an eval score to a win rate prediction by applying a [`sigmoid`].
pub fn cp_to_wr(cp: CpScore, eval_scale: ScalingFactor) -> WrScore {
    WrScore(sigmoid(cp.0, eval_scale))
}

/// The *loss* of a single sample.
///
/// The loss is a measure of how wrong our prediction is; smaller values are better.
/// This function computes the loss as the squared error, multiplied by the sampling weight.
pub fn sample_loss(wr_prediction: WrScore, outcome: Outcome, sample_weight: Float) -> Float {
    let delta = wr_prediction.0 - outcome.0;
    delta * delta * sample_weight
}

/// The loss of an eval score, see [sample_loss].
pub fn sample_loss_for_cp(
    eval: CpScore,
    outcome: Outcome,
    eval_scale: ScalingFactor,
    sample_weight: Float,
) -> Float {
    let wr_prediction = cp_to_wr(eval, eval_scale);
    sample_loss(wr_prediction, outcome, sample_weight)
}

/// The *gradient* of the loss function, based on a single sample.
///
/// Constant factors are ignored by this function.
/// Optimization works by changing weights into the opposite direction of the gradient.
pub fn scaled_sample_grad(prediction: WrScore, outcome: Outcome, sample_weight: Float) -> Float {
    (prediction.0 - outcome.0) * prediction.0 * (1.0 - prediction.0) * sample_weight
}

/// A single weight.
///
/// Tuning works by changing the values of all weights in parallel to minimize the loss.
#[derive(
    Debug,
    Display,
    Default,
    Copy,
    Clone,
    PartialOrd,
    PartialEq,
    Add,
    AddAssign,
    Sub,
    SubAssign,
    Mul,
    Div,
)]
pub struct Weight(pub Float);

impl Weight {
    /// Round this weight to the nearest integer.
    pub fn rounded(self) -> i32 {
        self.0.round() as i32
    }

    /// Convert this weight into a string of the rounded value.
    /// If `special` is [`true`], paint it red.
    pub fn to_string(self, special: bool) -> String {
        if special {
            format!("{}", self.0.round()).red().to_string()
        } else {
            format!("{}", self.0.round())
        }
    }
}

/// In the tuner, a position and the gradient are represented as a list of weights.
///
/// In an ideal world, this struct would take the number N of weights as a generic parameter.
/// However, const generics are very limited in (stable) Rust, which makes this a pain to implement.
/// So instead, the size is only known at runtime.
#[derive(Debug, Default, Clone, Deref, DerefMut)]
pub struct Weights(pub Vec<Weight>);

/// The gradient gives the opposite direction in which weights need to be changed to reduce the loss.
pub type Gradient = Weights;

impl Weights {
    /// Construct a list of `num_weights` weights, all initialized to zero.
    pub fn new(num_weights: usize) -> Self {
        Self(vec![Weight(0.0); num_weights])
    }

    /// The number of weights.
    pub fn num_weights(&self) -> usize {
        self.0.len()
    }
}

impl Display for Weights {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        assert!(!self.is_empty());
        write!(f, "[{}", self[0].0)?;
        for w in self.iter().skip(1) {
            write!(f, ", {}", w.0)?;
        }
        write!(f, "]")
    }
}

impl AddAssign<&Self> for Weights {
    fn add_assign(&mut self, rhs: &Self) {
        self.iter_mut().zip(rhs.iter()).for_each(|(a, b)| *a += *b)
    }
}

impl SubAssign<&Self> for Weights {
    fn sub_assign(&mut self, rhs: &Self) {
        self.iter_mut().zip(rhs.iter()).for_each(|(a, b)| *a -= *b)
    }
}

impl MulAssign<Float> for Weights {
    fn mul_assign(&mut self, rhs: Float) {
        for w in self.iter_mut() {
            w.0 *= rhs;
        }
    }
}

impl Mul<Float> for Weights {
    type Output = Self;

    fn mul(mut self, rhs: Float) -> Self::Output {
        self *= rhs;
        self
    }
}

impl DivAssign<Float> for Weights {
    fn div_assign(&mut self, rhs: Float) {
        for w in self.iter_mut() {
            w.0 /= rhs;
        }
    }
}

impl Div<Float> for Weights {
    type Output = Weights;

    fn div(mut self, rhs: Float) -> Self::Output {
        self /= rhs;
        self
    }
}

impl Weights {
    fn update<D: Datapoint>(&mut self, data_point: &D, factor: Float) {
        for feature in data_point.features() {
            self[feature.idx].0 += feature.weight * factor;
        }
    }
}

type FeatureT = i8;

/// A feature can occur some fixed number of times in a position.
///
/// For example, one possible feature would be "number of rooks" in a chess position.
/// This would be computed by subtracting the number of black rooks from the number of white rooks.
/// Then, for each position, all weights corresponding to this feature are multiplied by the feature count
/// and added up over all features to compute the [`CpScore`].
///
/// Because usually, most features will appear in a given position, the list of features is stored as a sparse array
/// (i.e. only non-zero features are actually stored).
/// Users should not generally have to deal with this type directly; building their `[trace]`(TraceTrait) on top of
/// `[SimpleTrace]` should take care of constructing this struct.
#[derive(Debug, Default, Copy, Clone, PartialOrd, PartialEq)]
pub struct Feature {
    feature: FeatureT,
    idx: u16,
}

impl Feature {
    /// Constructs a new feature.
    pub fn new(feature: FeatureT, idx: u16) -> Self {
        Self { feature, idx }
    }

    /// Converts the feature to a [`Float`].
    pub fn float(self) -> Float {
        self.feature as Float
    }
    /// The zero-based index of this feature.
    ///
    /// Note that a feature may correspond to more than one weight;
    /// [`TaperedDatapoint`] takes care of dealing with that.
    pub fn idx(self) -> usize {
        self.idx as usize
    }
}

/// A phased eval interpolates between middlegame and endgame [`Weight`]s using this multiplier.
///
/// This value should be in `[0, 1]`.
#[derive(Debug, Copy, Clone)]
pub struct PhaseMultiplier(Float);

/// A trace stores extracted features of a position and can be converted to a list of [`Feature`]s.
///
/// This type is returned by the [`feature_trace`](super::eval::Eval::feature_trace) method.
/// The simplest way to implement this trait is to make your strut contain several [`SimpleTrace`]s or other
/// pre-defined trace implementation, which do the actual work of converting the trace to a list of features.
///
/// For example:
/// ```
/// use pliers::gd::{Feature, Float, SimpleTrace, TraceNFeatures, TraceTrait};
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
        let mut offset = 0;
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
#[derive(Debug, Default)]
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
                let feature = Feature {
                    feature: diff as FeatureT,
                    idx: idx as u16,
                };
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

/// Struct used for tuning.
///
/// Each [`WeightedFeature`] of a [`Datapoint`] is multiplied by the corresponding current eval weight and added up
/// to compute the [`CpScore`]. Users should not generally need to worry about this, unless they want to implement
/// their own tuner.
pub struct WeightedFeature {
    /// The weight of this entry.
    pub weight: Float,
    /// The index of the *weight* that this entry corresponds to.
    /// This is not necessarily the same as the feature index if the eval is tapered.
    pub idx: usize,
}

impl WeightedFeature {
    fn new(idx: usize, weight: Float) -> Self {
        Self { weight, idx }
    }
}

/// Represents a single position.
///
/// A position is represented as a list of weighted features, an outcome, and a sampling weight (default: 1.0).
/// Note that this representation is completely independent of the actual game or evaluation function:
/// Once the feature counts have been computed (this happens when loading the data), no part of the tuning process
/// depends on the eval anymore, except for printing the current weights in a human-readable way.
pub trait Datapoint: Clone + Send + Sync {
    /// Creates a new [`Datapoint`] from a [trace](TraceTrait) and [outcome](Outcome).
    ///
    /// The `weight` is used for downweighting samples, but of the three provided trait implementations,
    /// only [`WeightedDatapoint`] cares about this. It should rarely be needed.
    fn new<T: TraceTrait>(trace: T, outcome: Outcome, weight: Float) -> Self;

    /// The outcome of this position, a [win rate prediction](Outcome) between `0` and `1`.
    fn outcome(&self) -> Outcome;

    /// The list of weighted features that appear in this position.
    ///
    /// This weight can depend on the general weight of this datapoint as well as on the phase tapering factor
    /// for a tapered eval.
    fn features(&self) -> impl Iterator<Item = WeightedFeature>;

    /// A value of 2.0 effectively duplicates this datapoint, so it will influence the gradient twice as much.
    fn sampling_weight(&self) -> Float {
        1.0
    }
}

/// A simple Datapoint that ignores phase and weight.
#[derive(Debug, Clone)]
pub struct NonTaperedDatapoint {
    /// The list of features.
    pub features: Vec<Feature>,
    /// The win rate prediction of the FEN (can be based on a WDL result or an engine's score).
    pub outcome: Outcome,
}

impl Datapoint for NonTaperedDatapoint {
    fn new<T: TraceTrait>(trace: T, outcome: Outcome, _weight: Float) -> Self {
        Self {
            features: trace.as_features(0),
            outcome,
        }
    }

    fn outcome(&self) -> Outcome {
        self.outcome
    }

    fn features(&self) -> impl Iterator<Item = WeightedFeature> {
        self.features
            .iter()
            .map(|feature| WeightedFeature::new(feature.idx(), feature.float()))
    }
}

/// A Datapoint where each feature corresponds to two weights, interpolated based on the game phase.
#[derive(Debug, Clone)]
pub struct TaperedDatapoint {
    /// The features of this position.
    pub features: Vec<Feature>,
    /// The win rate prediction of the FEN (can be based on the WDL result or an engine's score).
    pub outcome: Outcome,
    /// The game phase.
    pub phase: PhaseMultiplier,
}

impl Datapoint for TaperedDatapoint {
    fn new<T: TraceTrait>(trace: T, outcome: Outcome, _weight: Float) -> Self {
        Self {
            features: trace.as_features(0),
            outcome,
            phase: PhaseMultiplier(trace.phase()),
        }
    }

    fn outcome(&self) -> Outcome {
        self.outcome
    }

    fn features(&self) -> impl Iterator<Item = WeightedFeature> {
        self.features.iter().flat_map(|feature| {
            [
                WeightedFeature::new(feature.idx() * 2, feature.float() * self.phase.0),
                WeightedFeature::new(
                    feature.idx() * 2 + 1,
                    feature.float() * (1.0 - self.phase.0),
                ),
            ]
        })
    }
}

/// Like [TaperedDatapoint], but additionally holds a weight that can be used to signify how important this position is.
#[derive(Debug, Clone)]
pub struct WeightedDatapoint {
    /// The nested tapered datapoint.
    pub inner: TaperedDatapoint,
    /// The sample weight of the datapoint. Set through the JSON list of datasets or by the [`Filter`](super::load_data::Filter).
    pub weight: Float,
}

impl Datapoint for WeightedDatapoint {
    fn new<T: TraceTrait>(trace: T, outcome: Outcome, weight: Float) -> Self {
        Self {
            inner: TaperedDatapoint::new(trace, outcome, weight),
            weight,
        }
    }

    fn outcome(&self) -> Outcome {
        self.inner.outcome
    }

    fn features(&self) -> impl Iterator<Item = WeightedFeature> {
        self.inner.features()
    }

    fn sampling_weight(&self) -> Float {
        self.weight
    }
}

// TODO: Let `Dataset` own the list of features and `D` objects only hold a slice of features
/// The totality of all data points.
///
/// Most code should work with [`Batch`]es instead.
#[derive(Debug)]
pub struct Dataset<D: Datapoint> {
    datapoints: Vec<D>,
    weights_in_pos: usize,
    sampling_weight_sum: Float,
}

impl<D: Datapoint> Dataset<D> {
    /// Create a new dataset, where each data point consist of `num_weights` weights.
    pub fn new(num_weights: usize) -> Self {
        Self {
            datapoints: vec![],
            weights_in_pos: num_weights,
            sampling_weight_sum: 0.0,
        }
    }

    /// The number of weights per position.
    pub fn num_weights(&self) -> usize {
        self.weights_in_pos
    }

    /// Access the underlying array of data points.
    pub fn data(&self) -> &[D] {
        &self.datapoints
    }

    /// Add a new datapoint.
    pub fn push(&mut self, datapoint: D) {
        self.sampling_weight_sum += datapoint.sampling_weight();
        self.datapoints.push(datapoint);
    }

    /// Combine two datasets into one larger dataset without removing duplicate positions.
    pub fn union(&mut self, mut other: Dataset<D>) {
        assert_eq!(self.weights_in_pos, other.weights_in_pos);
        self.datapoints.append(&mut other.datapoints);
        self.sampling_weight_sum += other.sampling_weight_sum;
    }

    /// Shuffle the dataset, which is useful when not tuning on the entire dataset.
    pub fn shuffle(&mut self) {
        self.datapoints.shuffle(&mut thread_rng());
    }

    /// Converts the entire dataset into a single batch.
    pub fn as_batch(&self) -> Batch<D> {
        Batch {
            datapoints: &self.datapoints,
            num_weights: self.weights_in_pos,
            weight_sum: self.sampling_weight_sum,
        }
    }

    /// Turns a subset of the dataset into a batch.
    ///
    /// Note that this needs to compute the sum of sampling weights,
    /// which makes this an `O(n)` operation, where `n` is the size of the returned batch.
    pub fn batch(&self, start_idx: usize, end_idx: usize) -> Batch<D> {
        let datapoints = &self.datapoints[start_idx..end_idx];
        let weight_sum = datapoints.iter().map(|d| d.sampling_weight()).sum();
        Batch {
            datapoints,
            num_weights: self.weights_in_pos,
            weight_sum,
        }
    }
}

/// A list of data points on which the eval gets optimized.
#[derive(Debug)]
pub struct Batch<'a, D: Datapoint> {
    /// The underlying array of data points.
    pub datapoints: &'a [D],
    /// The number of weights per data point.
    pub num_weights: usize,
    /// The sum of sampling weights.
    ///
    /// If all positions have a sampling weight if 1.0 (the default),
    /// this is the same as the len of the `datapoints` slice.
    pub weight_sum: Float,
}

// deriving Copy, Clone doesn't work for some reason
impl<D: Datapoint> Clone for Batch<'_, D> {
    fn clone(&self) -> Self {
        Self {
            datapoints: self.datapoints,
            num_weights: self.num_weights,
            weight_sum: self.weight_sum,
        }
    }
}

impl<D: Datapoint> Copy for Batch<'_, D> {}

impl<'a, D: Datapoint> Deref for Batch<'a, D> {
    type Target = [D];

    fn deref(&self) -> &Self::Target {
        self.datapoints
    }
}

/// Eval of a position, given the current weights.
pub fn cp_eval_for_weights<D: Datapoint>(weights: &Weights, position: &D) -> CpScore {
    let mut res = 0.0;
    for feature in position.features() {
        res += feature.weight * weights[feature.idx].0;
    }
    CpScore(res)
}

/// Win rate prediction of a position, given the current weights.
pub fn wr_prediction_for_weights<D: Datapoint>(
    weights: &Weights,
    position: &D,
    eval_scale: ScalingFactor,
) -> WrScore {
    let eval = cp_eval_for_weights(weights, position);
    cp_to_wr(eval, eval_scale)
}

/// Loss of a position, given the current weights.
pub fn loss<D: Datapoint>(
    weights: &Weights,
    batch: Batch<'_, D>,
    eval_scale: ScalingFactor,
) -> Float {
    let sum = if batch.len() >= MIN_MULTITHREADING_BATCH_SIZE {
        batch
            .par_iter()
            .map(|datapoint| {
                let eval = wr_prediction_for_weights(weights, datapoint, eval_scale);
                let loss = sample_loss(eval, datapoint.outcome(), datapoint.sampling_weight());
                debug_assert!(loss >= 0.0);
                loss
            })
            .sum()
    } else {
        let mut res = Float::default();
        for datapoint in batch.iter() {
            let eval = wr_prediction_for_weights(weights, datapoint, eval_scale);
            let loss = sample_loss(eval, datapoint.outcome(), datapoint.sampling_weight());
            debug_assert!(loss >= 0.0);
            res += loss * datapoint.sampling_weight();
        }
        res
    };
    sum / batch.weight_sum as Float
}

/// Computes the gradient of the loss function over the entire batch.
///
/// The loss function of a single sample is `(sigmoid(sample, scale) - outcome) ^ 2`,
/// so per the chain rule, the derivative is `2 * (sigmoid(sample, scale) - outcome) * sigmoid'(sample, scale)`,
/// where the derivative of the sigmoid, sigmoid', is `scale * sigmoid(sample, scale) * (1 - sigmoid(sample, scale)`.
/// Even though constant factors don't matter for gradient descent, this function still returns the exact gradient,
/// without ignoring constant factors.
/// The computation gets parallelized if the batch exceeds a size of [`MIN_MULTITHREADING_BATCH_SIZE`].
pub fn compute_gradient<D: Datapoint>(
    weights: &Weights,
    batch: Batch<D>,
    eval_scale: ScalingFactor,
) -> Gradient {
    let constant_factor = 2.0 * eval_scale / batch.weight_sum;
    if batch.len() >= MIN_MULTITHREADING_BATCH_SIZE {
        batch
            .datapoints
            .par_iter()
            .fold(
                || Gradient::new(weights.num_weights()),
                |mut grad: Gradient, data: &D| {
                    let wr_prediction = wr_prediction_for_weights(weights, data, eval_scale);

                    // constant factors have been moved outside the loop
                    let scaled_delta =
                        scaled_sample_grad(wr_prediction, data.outcome(), data.sampling_weight());
                    grad.update(data, scaled_delta);
                    grad
                },
            )
            .reduce(
                || Gradient::new(weights.num_weights()),
                |mut a, b| {
                    a += &b;
                    a
                },
            )
            * constant_factor
    } else {
        let mut grad = Gradient::new(weights.num_weights());
        for data in batch.iter() {
            let wr_prediction = wr_prediction_for_weights(weights, data, eval_scale).0;

            // TODO: Multiply with `constant_factor` outside the loop?
            let scaled_delta = constant_factor
                * (wr_prediction - data.outcome().0)
                * wr_prediction
                * (1.0 - wr_prediction)
                * data.sampling_weight();
            grad.update(data, scaled_delta);
        }
        grad
    }
}

/// This is where the actual optimization happens.
///
/// Optimize the weights using the given [optimizer](Optimizer) for `num_epochs` epochs, where the gradient is computed
/// over the entire batch each epoch. Regularly prints the current weights using the supplied [weights interpretation](WeightsInterpretation].
pub fn optimize_entire_batch<D: Datapoint>(
    batch: Batch<D>,
    eval_scale: ScalingFactor,
    num_epochs: usize,
    weights_interpretation: &dyn WeightsInterpretation,
    optimizer: &mut dyn Optimizer<D>,
) -> Weights {
    let mut prev_weights: Vec<Weight> = vec![];
    let mut weights = Weights::new(batch.num_weights);
    if weights_interpretation.retune_from_zero() {
        // Since weights are initially 0, use a very high lr for the first couple of iterations.
        optimizer.lr_drop(0.25); // increases lr by a factor of
    } else {
        weights = weights_interpretation
            .initial_weights()
            .expect("if `retune_from_zero()` returns `false`, there must be initial weights");
        assert_eq!(
            weights.num_weights(),
            batch.num_weights,
            "Incorrect number of initial weights. Maybe your `Eval::NUM_WEIGHTS` is incorrect or your initial_weights() returns incorrect weights?"
        );
    }
    let mut prev_loss = Float::INFINITY;
    let start = Instant::now();
    for epoch in 0..num_epochs {
        optimizer.iteration(&mut weights, batch, eval_scale, epoch);
        if epoch % 50 == 0 {
            let loss = loss(&weights, batch, eval_scale);
            println!(
                "Epoch {epoch} complete, weights:\n {}",
                display(weights_interpretation, &weights, &prev_weights)
            );
            let elapsed = start.elapsed();
            // If no weight changed by more than 0.05 within the last 50 epochs, stop.
            let mut max_diff: Float = 0.0;
            for i in 0..prev_weights.len() {
                let diff = weights[i].0 - prev_weights[i].0;
                if diff.abs() > max_diff.abs() {
                    max_diff = diff;
                }
            }
            println!(
                "[{elapsed}s] Epoch {epoch} ({0:.1} epochs/s), loss: {loss}, loss got smaller by: 1/1_000_000 * {1}, \
                maximum weight change to 50 epochs ago: {max_diff:.2}",
                epoch as f32 / elapsed.as_secs_f32(),
                (prev_loss - loss) * 1_000_000.0,
                elapsed = elapsed.as_secs()
            );
            if loss <= 0.001 && epoch >= 20 {
                break;
            }
            if max_diff.abs() <= 0.05 && epoch >= 50 {
                break;
            }
            prev_weights.clone_from(&weights.0);
            prev_loss = loss;
        }
        if epoch == 20.min(num_epochs / 100) {
            optimizer.lr_drop(4.0); // undo the raised lr.
        } else if epoch == num_epochs * 3 / 4 {
            optimizer.lr_drop(1.5);
        }
    }
    weights
}

/// Convenience function for optimizing with the [`Adam`] optimizer.
fn adam_optimize<D: Datapoint>(
    batch: Batch<D>,
    eval_scale: ScalingFactor,
    num_epochs: usize,
    format_weights: &dyn WeightsInterpretation,
) -> Weights {
    optimize_entire_batch(
        batch,
        eval_scale,
        num_epochs,
        format_weights,
        &mut Adam::new(batch, eval_scale),
    )
}

/// Print the final weights once the optimization is complete.
///
/// Unlike the intermediate steps, this also prints how often each feature occurred, and optionally
/// interpolates the tuned weights with the initial weights based on this sample count.
pub fn print_optimized_weights<D: Datapoint>(
    weights: &Weights,
    batch: Batch<D>,
    scale: ScalingFactor,
    interpretation: &dyn WeightsInterpretation,
) {
    let occurrence_counts = count_occurrences(batch);
    let occurrences = Weights(occurrence_counts.iter().map(|o| Weight(*o)).collect());
    println!(
        "Occurrences:\n{}",
        display(interpretation, &occurrences, &[])
    );
    let mut weights = weights.clone();
    interpolate(&occurrence_counts, &mut weights, interpretation);
    println!(
        "Scaling factor: {scale:.2}, Final eval:\n{}",
        display(interpretation, &weights, &[])
    );
}

/// Change the current weights each iteration by taking into account the gradient.
///
/// Different implementations mostly differ in their step size control.
pub trait Optimizer<D: Datapoint> {
    /// Create a new optimizer.
    ///
    /// The [`Batch`] and [`ScalingFactor`] can be used to set internal hyperparameters.
    fn new(batch: Batch<D>, eval_scale: ScalingFactor) -> Self
    where
        Self: Sized;

    /// Can be less than 1 to increase the learning rate.
    fn lr_drop(&mut self, factor: Float);

    /// A single iteration of the optimizer.
    fn iteration(
        &mut self,
        weights: &mut Weights,
        batch: Batch<'_, D>,
        eval_scale: ScalingFactor,
        i: usize,
    );

    /// A simple but generic optimization procedure. Usually, calling [`optimize_entire_batch`] (directly or through
    /// the [`optimize`](super::optimize) function) results in faster convergence. This function is primarily useful for debugging.
    fn optimize_simple(
        &mut self,
        batch: Batch<'_, D>,
        eval_scale: ScalingFactor,
        num_iterations: usize,
    ) -> Weights {
        let mut weights = Weights::new(batch.num_weights);
        for i in 0..num_iterations {
            self.iteration(&mut weights, batch, eval_scale, i);
        }
        weights
    }
}

/// Gradient Descent optimizer that simply multiplies the gradient by the current learning rate `alpha`.
pub struct SimpleGDOptimizer {
    /// The learning rate.
    pub alpha: Float,
}

impl<D: Datapoint> Optimizer<D> for SimpleGDOptimizer {
    fn new(_batch: Batch<D>, eval_scale: ScalingFactor) -> Self {
        Self {
            alpha: eval_scale / 4.0,
        }
    }

    fn lr_drop(&mut self, factor: Float) {
        self.alpha /= factor;
    }

    fn iteration(
        &mut self,
        weights: &mut Weights,
        batch: Batch<D>,
        eval_scale: ScalingFactor,
        _i: usize,
    ) {
        let gradient = compute_gradient(weights, batch, eval_scale);
        for i in 0..weights.len() {
            weights[i].0 -= gradient[i].0 * self.alpha;
        }
    }
}

/// Hyperparameters are parameters that control the optimization process and are not themselves
/// automatically optimized.
#[derive(Debug, Copy, Clone)]
pub struct AdamHyperParams {
    /// Learning rate multiplier, an upper bound on the step size.
    pub alpha: Float,
    /// Exponential decay of the moving average of the gradient
    pub beta1: Float,
    /// Exponential decay of the moving average of the uncentered variance of the gradient
    pub beta2: Float,
    /// Offset to avoid division by zero
    pub epsilon: Float,
}

impl AdamHyperParams {
    fn for_eval_scale(eval_scale: ScalingFactor) -> Self {
        Self {
            alpha: eval_scale / 10.0,
            // Setting these values too low can introduce crazy swings in the eval values and loss when it would
            // otherwise appear converged -- maybe because of numerical instability?
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1e-8,
        }
    }
}

/// The default tuner, an implementation of the very widely used [Adam](https://arxiv.org/abs/1412.6980) optimizer.
#[derive(Debug)]
pub struct Adam {
    /// Hyperparameters. Should be set before starting to optimize.
    pub hyper_params: AdamHyperParams,
    /// first moment (exponentially moving average)
    m: Weights,
    /// second moment (exponentially moving average)
    v: Weights,
}

impl<D: Datapoint> Optimizer<D> for Adam {
    fn new(batch: Batch<D>, eval_scale: ScalingFactor) -> Self {
        let hyper_params = AdamHyperParams::for_eval_scale(eval_scale);
        Self {
            hyper_params,
            m: Weights::new(batch.num_weights),
            v: Weights::new(batch.num_weights),
        }
    }

    fn lr_drop(&mut self, factor: Float) {
        self.hyper_params.alpha /= factor;
    }

    fn iteration(
        &mut self,
        weights: &mut Weights,
        batch: Batch<D>,
        eval_scale: ScalingFactor,
        iteration: usize,
    ) {
        let iteration = iteration + 1;
        let beta1 = self.hyper_params.beta1;
        let beta2 = self.hyper_params.beta2;
        let gradient = compute_gradient(weights, batch, eval_scale);
        for i in 0..gradient.len() {
            // biased since the values are initialized to 0, so the exponential moving average is wrong
            self.m[i] = self.m[i] * beta1 + gradient[i] * (1.0 - beta1);
            self.v[i] = self.v[i] * beta2 + gradient[i] * gradient[i].0 * (1.0 - beta2);
            let unbiased_m = self.m[i] / (1.0 - beta1.powi(iteration as i32));
            let unbiased_v = self.v[i] / (1.0 - beta2.powi(iteration as i32));
            weights[i] -= unbiased_m * self.hyper_params.alpha
                / (unbiased_v.0.sqrt() + self.hyper_params.epsilon)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::distributions::{Distribution, Uniform};
    use rand::thread_rng;
    use std::cmp::Ordering::Equal;

    #[test]
    pub fn simple_loss_test() {
        let weights = Weights(vec![Weight(0.0); 42]);
        let no_features = vec![Feature::default(); 42];
        for outcome in [0.0, 0.5, 1.0] {
            let dataset = vec![NonTaperedDatapoint {
                features: no_features.clone(),
                outcome: Outcome::new(outcome),
            }];
            let batch = Batch {
                datapoints: dataset.as_slice(),
                num_weights: 1,
                weight_sum: 1.0,
            };
            for eval_scale in 1..100 {
                let loss = loss(&weights, batch, eval_scale as ScalingFactor);
                if outcome == 0.5 {
                    assert_eq!(loss, 0.0);
                } else {
                    assert!((loss - 0.25).abs() <= 0.0001, "{loss} {outcome}");
                }
            }
        }
    }

    #[test]
    pub fn compute_gradient_test() {
        let weights = Weights(vec![Weight(0.0)]);
        for outcome in [0.0_f64, 0.5, 1.0] {
            let data_points = [NonTaperedDatapoint {
                features: vec![Feature::new(1, 0)],
                outcome: Outcome::new(1.0),
            }];
            let batch = Batch {
                datapoints: data_points.as_slice(),
                num_weights: 1,
                weight_sum: 1.0,
            };
            let gradient = compute_gradient(&weights, batch, 1.0);
            assert_eq!(gradient.len(), 1);
            let gradient = gradient[0].0;
            assert_eq!(-gradient, 0.5 * 0.5 * 0.5 * 2.0 * outcome.signum());
        }
    }

    #[test]
    // testcase that contains only 1 position with only 1 feature
    pub fn trivial_test() {
        let scaling_factor = 100.0;
        for feature in [1, 2, -1, 0] {
            for initial_weight in [0.0, 0.1, 100.0, -1.2] {
                for outcome in [0.0, 0.5, 1.0, 0.9, 0.499] {
                    let mut weights = Weights(vec![Weight(initial_weight)]);
                    let position = vec![Feature::new(feature, 0)];
                    let datapoint = NonTaperedDatapoint {
                        features: position.clone(),
                        outcome: Outcome::new(outcome),
                    };
                    let mut dataset = Dataset::new(1);
                    dataset.push(datapoint);
                    let batch = dataset.as_batch();
                    for _ in 0..100 {
                        let grad = compute_gradient(&weights, batch, scaling_factor);
                        let old_weights = weights.clone();
                        weights -= &grad;
                        // println!("loss {0}, initial weight {initial_weight}, weights {weights}, gradient {grad}, eval {1}, predicted {2}, outcome {outcome}, feature {feature}, scaling factor {scaling_factor}", loss(&weights, &dataset, scaling_factor), cp_eval_for_weights(&weights, &dataset[0].position), wr_prediction_for_weights(&weights, &dataset[0].position, scaling_factor));
                        if initial_weight == 0.0 && grad.0[0].0.abs() > 0.0000001 {
                            assert_eq!(
                                weights.0[0].0.partial_cmp(&old_weights[0].0),
                                outcome.partial_cmp(&0.5).map(|x| if feature < 0 {
                                    x.reverse()
                                } else if feature == 0 {
                                    Equal
                                } else {
                                    x
                                })
                            );
                        }
                        let new_loss = loss(&weights, batch, scaling_factor);
                        let old_loss = loss(&old_weights, batch, scaling_factor);
                        assert!(new_loss >= 0.0, "{new_loss}");
                        assert!(new_loss <= old_loss, "new loss: {new_loss}, old loss: {old_loss}, feature {feature}, initial weight {initial_weight}, outcome {outcome}");
                    }
                    let loss = loss(&weights, batch, scaling_factor);
                    if feature != 0 {
                        // pure gradient descent with a small scaling factor can take some time to converge
                        assert!(
                            loss <= 0.01, /* * initial_weight.max(0.1)*/
                            "loss {loss}, initial weight {initial_weight}, weights {weights}, feature {feature}, outcome {outcome}, scaling factor {scaling_factor}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    pub fn simple_test() {
        for outcome in [0.0, 0.5, 1.0] {
            let mut weights = Weights(vec![Weight(0.0), Weight(1.0), Weight(-1.0)]);
            let position = vec![Feature::new(1, 0), Feature::new(-1, 1), Feature::new(2, 2)];
            let datapoint = NonTaperedDatapoint {
                features: position.clone(),
                outcome: Outcome::new(outcome),
            };
            let dataset = vec![datapoint];
            let batch = Batch {
                datapoints: dataset.as_slice(),
                num_weights: 3,
                weight_sum: 3.0,
            };
            for _ in 0..100 {
                let grad = compute_gradient(&weights, batch, 1.0);
                let old_weights = weights.clone();
                weights -= &grad;
                assert!(loss(&weights, batch, 1.0) <= loss(&old_weights, batch, 1.0));
            }
            let loss = loss(&weights, batch, 1.0);
            assert!(loss <= 0.01);
        }
    }

    #[test]
    pub fn two_features_test() {
        let scale = 500.0;
        for outcome in [0.0, 0.5, 1.0] {
            let mut weights = Weights(vec![Weight(123.987), Weight(-987.123)]);
            let position = vec![Feature::new(3, 0), Feature::new(-3, 1)];
            let datapoint = NonTaperedDatapoint {
                features: position.clone(),
                outcome: Outcome::new(outcome),
            };
            let dataset = vec![datapoint];
            let batch = Batch {
                datapoints: dataset.as_slice(),
                num_weights: 1,
                weight_sum: 1.0,
            };
            let mut lr = 1.0;
            for _ in 0..100 {
                let grad = compute_gradient(&weights, batch, scale);
                let old_weights = weights.clone();
                weights -= &(grad.clone() * lr);
                let current_loss = loss(&weights, batch, scale);
                let old_loss = loss(&old_weights, batch, scale);
                assert!(current_loss <= old_loss);
                lr *= 0.99;
            }
            let loss = loss(&weights, batch, scale);
            assert!(loss <= 0.01, "{loss}");
            if outcome == 0.5 {
                assert_eq!(weights[0].0.signum(), weights[1].0.signum());
                assert!((weights[0].0.abs() - weights[1].0.abs()).abs() <= 0.00000001);
            } else {
                assert_eq!(weights[0].0 > weights[1].0, outcome > 0.5);
            }
        }
    }

    #[test]
    pub fn two_positions_test() {
        let scale = 1000.0;
        let win = NonTaperedDatapoint {
            features: vec![Feature::new(1, 0), Feature::new(-1, 1)],
            outcome: WrScore(1.0),
        };
        let lose = NonTaperedDatapoint {
            features: vec![Feature::new(-1, 0), Feature::new(1, 1)],
            outcome: WrScore(0.0),
        };
        let dataset = vec![win, lose];
        let batch = Batch {
            datapoints: dataset.as_slice(),
            num_weights: 2,
            weight_sum: 2.0,
        };
        let weights_dist = Uniform::new(-100.0, 100.0);
        let mut rng = thread_rng();
        for _ in 0..100 {
            let mut weights = Weights(vec![
                Weight(weights_dist.sample(&mut rng)),
                Weight(weights_dist.sample(&mut rng)),
            ]);
            let mut weights_copy = weights.clone();
            for _ in 0..200 {
                let grad = compute_gradient(&weights, batch, scale);
                weights -= &grad;
            }
            let remaining_loss = loss(&weights, batch, scale);
            assert!(remaining_loss <= 0.001);
            assert!(weights[0].0 >= 100.0);
            assert!(weights[1].0 <= -100.0);

            type AnyOptimizer = Box<dyn Optimizer<NonTaperedDatapoint>>;
            let optimizers: [AnyOptimizer; 2] = [
                Box::new(SimpleGDOptimizer { alpha: 1.0 }),
                Box::new(Adam::new(batch, scale)),
            ];
            for mut optimizer in optimizers {
                for i in 0..200 {
                    optimizer.iteration(&mut weights_copy, batch, scale, i);
                }
                let remaining_loss = loss(&weights_copy, batch, scale);
                assert!(remaining_loss <= 0.001);
                assert!(weights[0].0 >= 100.0);
                assert!(weights[1].0 <= -100.0);
            }
        }
    }

    #[test]
    pub fn three_positions_test() {
        let mut weights = Weights(vec![Weight(0.4), Weight(1.0), Weight(2.0)]);
        let draw_datapoint = NonTaperedDatapoint {
            features: vec![Feature::new(0, 0), Feature::new(0, 1), Feature::new(0, 2)],
            outcome: Outcome::new(0.5),
        };
        let lose_datapoint = NonTaperedDatapoint {
            features: vec![Feature::new(-1, 0), Feature::new(-1, 1), Feature::new(0, 2)],
            outcome: Outcome::new(0.0),
        };
        let win_datapoint = NonTaperedDatapoint {
            features: vec![Feature::new(1, 0), Feature::new(1, 1), Feature::new(0, 2)],
            outcome: Outcome::new(1.0),
        };

        let dataset = vec![draw_datapoint, win_datapoint, lose_datapoint];
        let batch = Batch {
            datapoints: dataset.as_slice(),
            num_weights: 3,
            weight_sum: 3.0,
        };
        for _ in 0..500 {
            let grad = compute_gradient(&weights, batch, 1.0);
            println!(
                "current weights: {0}, current loss: {1}, gradient: {2}",
                weights,
                loss(&weights, batch, 1.0),
                grad,
            );
            let old_weights = weights.clone();
            weights -= &grad;
            assert!(loss(&weights, batch, 1.0) <= loss(&old_weights, batch, 1.0));
        }
        assert!(weights[0].0 >= 0.0);
        assert!(weights[1].0 >= 0.0);
        assert!(
            weights[1].0 - weights[0].0 <= 0.6 + 0.001,
            "difference in duplicated weights is not supposed to get larger"
        );
        assert_eq!(
            weights[2].0, 2.0,
            "irrelevant weight is not supposed to change at all"
        );
    }

    #[test]
    pub fn adam_one_weight_test() {
        for outcome in [0.0, 0.5, 1.0] {
            let eval_scale = 10000.0;
            let dataset = vec![NonTaperedDatapoint {
                features: vec![Feature::new(1, 0)],
                outcome: Outcome::new(outcome),
            }];
            let batch = Batch {
                datapoints: dataset.as_slice(),
                num_weights: 1,
                weight_sum: 1.0,
            };
            let mut adam = Adam::new(batch, eval_scale);
            let weights = adam.optimize_simple(batch, eval_scale, 20);
            assert_eq!(weights.len(), 1);
            let weight = weights[0].0;
            assert_eq!(weight.signum(), (outcome - 0.5).signum());
            if outcome == 1.0 {
                assert!(weight >= 10.0);
            } else if outcome == 0.0 {
                assert!(weight <= -10.0);
            }
        }
    }
}
