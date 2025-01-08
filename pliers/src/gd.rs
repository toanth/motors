//! Everything related to the actual optimization, using a Gradient Descent-based tuner ([`Adam`] by default).

use crate::eval::{count_occurrences, display, interpolate, WeightsInterpretation};
use crate::trace::TraceTrait;
use derive_more::{Add, AddAssign, Deref, DerefMut, Display, Div, Mul, Sub, SubAssign};
use gears::colored::Colorize;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use rayon::prelude::*;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::ops::{DivAssign, MulAssign};
use std::time::Instant;

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
#[must_use]
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

/// Larger loss values mean that the prediction is less accurate.
pub trait LossFn: Fn(WrScore, Outcome) -> Float + Sync + Copy {}

impl<T: Fn(WrScore, Outcome) -> Float + Sync + Copy> LossFn for T {}

/// The *loss* of a single sample.
///
/// The loss is a measure of how wrong our prediction is; smaller values are better.
/// Apart from optimizing the scaling factor, the loss itself is only used for displaying it to the user,
/// only the derivative is used for optimization.
/// For displaying a loss, it often makes more sense to use the quadratic sample loss:
/// - Under somewhat reasonable assumptions, minimizing the cross-entropy loss is equivalent to minimizing the quadratic loss
/// - The quadratic loss is always zero for a perfect prediction, unlike the cross-entropy loss
/// - The quadratic loss is slightly cheaper to compute
pub fn default_sample_loss(wr_prediction: WrScore, outcome: Outcome) -> Float {
    quadratic_sample_loss(wr_prediction, outcome)
}

/// The quadratic sample is loss is the square of `wr_prediction - outcome)`.
///
/// Unlike the [`cross_entropy_sample_loss`], it is always zero if a prediction perfectly matches the outcome.
pub fn quadratic_sample_loss(wr_prediction: WrScore, outcome: Outcome) -> Float {
    let delta = wr_prediction.0 - outcome.0;
    delta * delta
}

/// The cross-entropy is a good choice when optimizing anything where the output is a sigmoid, but it has some
/// undesirable properties.
// TODO: Test if this is bugged?
pub fn cross_entropy_sample_loss(wr_prediction: WrScore, outcome: Outcome) -> Float {
    let expected = outcome.0;
    let epsilon = 1e-8;
    let x = wr_prediction.0 * (1.0 - 2.0 * epsilon) + epsilon;
    let res = -(expected * x.ln() + (1.0 - expected) * (1.0 - x).ln());
    assert!(!res.is_nan());
    res
}

/// The *gradient* of the loss function and sigmoid, based on a single sample.
///
/// Constant factors are ignored by this function.
/// Optimization works by changing weights into the opposite direction of the gradient.
pub trait LossGradient: Sync + Copy {
    /// Compute the gradient of the loss of the sigmoid of a single sample.
    fn sample_gradient(score: WrScore, outcome: Outcome) -> Float;
}

/// The gradient of the quadratic loss applied to the sigmoid of the cp eval.
/// This may give slightly better results than the cross-entropy loss, but it can take a lot longer to converge
#[derive(Debug, Default, Copy, Clone)]
pub struct QuadraticLoss {}

impl LossGradient for QuadraticLoss {
    fn sample_gradient(prediction: WrScore, outcome: Outcome) -> Float {
        (prediction.0 - outcome.0) * prediction.0 * (1.0 - prediction.0)
    }
}

/// The cross-entropy loss. This can sometimes lead to faster convergence than the quadratic loss.
#[derive(Debug, Default, Copy, Clone)]
pub struct CrossEntropyLoss {}

impl LossGradient for CrossEntropyLoss {
    /// The gradient of the cross-entropy loss of the sigmoid of the cp eval. See [`scaled_sample_grad`].
    /// This is  `d/deval loss(prediction) = d/deval loss(sigmoid(eval, scaling_factor))`.
    /// Since `loss` is the cross-entropy loss, this cancels out to `(prediction.0 - outcome.0) * sample_weight`
    fn sample_gradient(prediction: WrScore, outcome: Outcome) -> Float {
        prediction.0 - outcome.0
    }
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
    #[must_use]
    pub fn rounded(self) -> i32 {
        self.0.round() as i32
    }

    /// Convert this weight into a string of the rounded value.
    /// If `special` is [`true`], paint it red.
    /// The string takes up at least `width` characters
    pub fn to_string(self, special: bool, width: usize) -> String {
        if special {
            format!("{:width$}", self.0.round()).red().to_string()
        } else {
            format!("{:width$}", self.0.round())
        }
    }
}

/// In the tuner, a position and the gradient are represented as a list of weights.
///
/// In an ideal world, this struct would take the number N of weights as a generic parameter.
/// However, const generics are very limited in (stable) Rust, which makes this a pain to implement.
/// So instead, the size is only known at runtime.
#[derive(Debug, Default, Clone, Deref, DerefMut)]
#[must_use]
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
        self.iter_mut().zip(rhs.iter()).for_each(|(a, b)| *a += *b);
    }
}

impl SubAssign<&Self> for Weights {
    fn sub_assign(&mut self, rhs: &Self) {
        self.iter_mut().zip(rhs.iter()).for_each(|(a, b)| *a -= *b);
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

pub(super) type FeatureT = i8;

/// A feature can occur some fixed number of times in a position.
///
/// For example, one possible feature would be "number of rooks" in a chess position.
/// This would be computed by subtracting the number of black rooks from the number of white rooks.
/// Then, for each position, all weights corresponding to this feature are multiplied by the feature count
/// and added up over all features to compute the [`CpScore`].
///
/// Because usually, most features will not appear in a given position, the list of features is stored as a sparse array
/// (i.e. only non-zero features are actually stored).
/// Users should not generally have to deal with this type directly; building their [`trace`](TraceTrait) on top of
/// [`SimpleTrace`] should take care of constructing this struct.
#[derive(Debug, Default, Copy, Clone, PartialOrd, PartialEq)]
#[must_use]
pub struct Feature {
    count: FeatureT,
    idx: u16,
}

impl Feature {
    /// Constructs a new feature.
    pub fn new(count: FeatureT, idx: u16) -> Self {
        Self { count, idx }
    }

    /// Converts the feature to a [`Float`].
    pub fn float(self) -> Float {
        self.count as Float
    }
    /// The zero-based index of this feature.
    ///
    /// Note that a feature may correspond to more than one weight;
    /// [`TaperedDatapoint`] takes care of dealing with that.
    pub fn idx(self) -> usize {
        self.idx as usize
    }
}

/// Struct used for tuning.
///
/// Each [`WeightedFeature`] of a [`Datapoint`] is multiplied by the corresponding current eval weight and added up
/// to compute the [`CpScore`]. Users should not generally need to worry about this, unless they want to implement
/// their own tuning algorithm.
#[derive(Debug, Copy, Clone)]
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
    fn new<T: TraceTrait>(trace: T, outcome: Outcome, weight: Float) -> Self {
        Self::new_from_features(trace.as_features(0), trace.phase(), outcome, weight)
    }

    /// Create a new datapoint from a list of features, phase, outcome and weight (only needed for [`WeightedDatapoint`]).
    fn new_from_features(
        features: Vec<Feature>,
        phase: Float,
        outcome: Outcome,
        weight: Float,
    ) -> Self;

    /// The outcome of this position, a [win rate prediction](Outcome) between `0` and `1`.
    fn outcome(&self) -> Outcome;

    /// The list of weighted features that appear in this position.
    ///
    /// This weight can depend on the general weight of this datapoint as well as on the phase tapering factor
    /// for a tapered eval.
    fn features(&self) -> impl Iterator<Item = WeightedFeature>;

    /// Into how many weights does a feature get transformed? For a [`TaperedDatapoint`], this is the number of phases (2),
    /// for a non-tapered datapoint this is 1.
    fn num_weights_per_feature() -> usize;

    /// In most cases, sampling weights are unused, and this enables some optimizations.
    fn all_sampling_weights_identical() -> bool {
        true
    }

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
    fn new_from_features(
        features: Vec<Feature>,
        _phase: Float,
        outcome: Outcome,
        _weight: Float,
    ) -> Self {
        Self { features, outcome }
    }

    fn outcome(&self) -> Outcome {
        self.outcome
    }

    fn features(&self) -> impl Iterator<Item = WeightedFeature> {
        self.features
            .iter()
            .map(|feature| WeightedFeature::new(feature.idx(), feature.float()))
    }

    fn num_weights_per_feature() -> usize {
        1
    }
}

/// A Datapoint where each feature corresponds to two weights, interpolated based on the game phase.
/// This should be the go-to type for most hand-crafted eval functions.
#[derive(Debug, Clone)]
pub struct TaperedDatapoint {
    /// The features of this position.
    #[cfg(not(feature = "save_space"))]
    features: Vec<WeightedFeature>,
    #[cfg(feature = "save_space")]
    features: Vec<Feature>,
    #[cfg(feature = "save_space")]
    phase: Float,
    /// The win rate prediction of the FEN (can be based on the WDL result or an engine's score).
    outcome: Outcome,
}

impl Datapoint for TaperedDatapoint {
    #[cfg(feature = "save_space")]
    fn new_from_features(
        features: Vec<Feature>,
        phase: Float,
        outcome: Outcome,
        _weight: Float,
    ) -> Self {
        Self {
            features,
            phase,
            outcome,
        }
    }

    #[cfg(not(feature = "save_space"))]
    fn new_from_features(
        features: Vec<Feature>,
        phase: Float,
        outcome: Outcome,
        _weight: Float,
    ) -> Self {
        // Doing this here instead of "on demand" in [`features`] dramatically improves performance
        // while still keeping memory requirements manageable
        let features = features
            .into_iter()
            .flat_map(|feature| {
                [
                    WeightedFeature::new(feature.idx() * 2, feature.float() * phase),
                    WeightedFeature::new(feature.idx() * 2 + 1, feature.float() * (1.0 - phase)),
                ]
            })
            .collect();
        Self { features, outcome }
    }

    fn outcome(&self) -> Outcome {
        self.outcome
    }

    #[cfg(feature = "save_space")]
    fn features(&self) -> impl Iterator<Item = WeightedFeature> {
        self.features.iter().flat_map(|feature| {
            [
                WeightedFeature::new(feature.idx() * 2, feature.float() * self.phase),
                WeightedFeature::new(feature.idx() * 2 + 1, feature.float() * (1.0 - self.phase)),
            ]
        })
    }

    #[cfg(not(feature = "save_space"))]
    fn features(&self) -> impl Iterator<Item = WeightedFeature> {
        self.features.iter().copied()
    }

    fn num_weights_per_feature() -> usize {
        2
    }
}

/// Like [`TaperedDatapoint`], but additionally holds a weight that can be used to signify how important this position is.
#[derive(Debug, Clone)]
pub struct WeightedDatapoint {
    /// The nested tapered datapoint.
    pub inner: TaperedDatapoint,
    /// The sample weight of the datapoint. Set through the JSON list of datasets or by the [`Filter`](super::load_data::Filter).
    pub weight: Float,
}

impl Datapoint for WeightedDatapoint {
    fn new_from_features(
        features: Vec<Feature>,
        phase: Float,
        outcome: Outcome,
        weight: Float,
    ) -> Self {
        Self {
            inner: TaperedDatapoint::new_from_features(features, phase, outcome, weight),
            weight,
        }
    }

    fn outcome(&self) -> Outcome {
        self.inner.outcome
    }

    fn features(&self) -> impl Iterator<Item = WeightedFeature> {
        self.inner.features()
    }

    fn num_weights_per_feature() -> usize {
        TaperedDatapoint::num_weights_per_feature()
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
#[must_use]
pub struct Dataset<D: Datapoint> {
    data_points: Vec<D>,
    weights_in_pos: usize,
    sampling_weight_sum: Float,
}

impl<D: Datapoint> Dataset<D> {
    /// Create a new dataset, where each data point consist of `num_weights` weights.
    pub fn new(num_weights: usize) -> Self {
        Self {
            data_points: vec![],
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
        &self.data_points
    }

    /// Add a new datapoint.
    pub fn push(&mut self, datapoint: D) {
        self.sampling_weight_sum += datapoint.sampling_weight();
        self.data_points.push(datapoint);
    }

    /// Combine two datasets into one larger dataset without removing duplicate positions.
    pub fn union(&mut self, mut other: Dataset<D>) {
        assert_eq!(self.weights_in_pos, other.weights_in_pos);
        self.data_points.append(&mut other.data_points);
        self.sampling_weight_sum += other.sampling_weight_sum;
    }

    /// Shuffle the dataset, which is useful when not tuning on the entire dataset.
    pub fn shuffle(&mut self) {
        self.data_points.shuffle(&mut thread_rng());
    }

    /// Converts the entire dataset into a single batch.
    pub fn as_batch(&self) -> Batch<D> {
        Batch {
            datapoints: &self.data_points,
            num_weights: self.weights_in_pos,
            weight_sum: self.sampling_weight_sum,
        }
    }

    /// Turns a subset of the dataset into a batch.
    ///
    /// Note that unless `D::all_sampling_weights_identical()` returns true, this needs to compute the sum of sampling weights,
    /// which makes this an `O(n)` operation, where `n` is the size of the returned batch.
    pub fn batch(&self, start_idx: usize, end_idx: usize) -> Batch<D> {
        let end_idx = end_idx.min(self.data_points.len());
        let datapoints = &self.data_points[start_idx..end_idx];
        let weight_sum = if D::all_sampling_weights_identical() {
            datapoints
                .first()
                .map(|d| d.sampling_weight())
                .unwrap_or(1.0)
                * datapoints.len() as Float
        } else {
            datapoints.iter().map(Datapoint::sampling_weight).sum()
        };
        Batch {
            datapoints,
            num_weights: self.weights_in_pos,
            weight_sum,
        }
    }
}

/// A list of data points on which the eval gets optimized.
#[derive(Debug, Clone)]
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

// deriving `Copy` doesn't work for some reason, because apparently `D` would have to be copyable for that?
impl<D: Datapoint> Copy for Batch<'_, D> {}

impl<D: Datapoint> Deref for Batch<'_, D> {
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
    loss_for(weights, batch, eval_scale, quadratic_sample_loss)
}

/// Loss of a position, given the current weights, using the `sample_loss` parameter to calculate
/// the loss of a single sample.
pub fn loss_for<D: Datapoint, L: LossFn>(
    weights: &Weights,
    batch: Batch<'_, D>,
    eval_scale: ScalingFactor,
    sample_loss: L,
) -> Float {
    let sum = if batch.len() >= MIN_MULTITHREADING_BATCH_SIZE {
        batch
            .par_iter()
            .map(|datapoint| {
                let eval = wr_prediction_for_weights(weights, datapoint, eval_scale);
                let loss = sample_loss(eval, datapoint.outcome()) * datapoint.sampling_weight();
                debug_assert!(loss >= 0.0);
                loss
            })
            .sum()
    } else {
        let mut res = Float::default();
        for datapoint in batch.iter() {
            let eval = wr_prediction_for_weights(weights, datapoint, eval_scale);
            let loss = sample_loss(eval, datapoint.outcome()) * datapoint.sampling_weight();
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
/// where the derivative of the sigmoid, sigmoid', is `1 / scale * sigmoid(sample, scale) * (1 - sigmoid(sample, scale)`.
/// However, this function multiplies by `scale` instead of `1/scale`: If the scale is larger, we need correspondingly
/// larger changes in the weights to see the same effect, even though the gradient is scaled down instead of up by that
/// factor. Apart from that, thi function returns the correct gradient, i.e. the actual gradient can be recovered by
/// dividing by `eval_scale * eval_scale`.
/// The computation gets parallelized if the batch exceeds a size of [`MIN_MULTITHREADING_BATCH_SIZE`].
pub fn compute_scaled_gradient_with<D: Datapoint, G: LossGradient>(
    weights: &Weights,
    batch: Batch<D>,
    eval_scale: ScalingFactor,
    _loss: G,
) -> Gradient {
    compute_scaled_gradient::<D, G>(weights, batch, eval_scale)
}

/// Computes the scaled gradient (see [`compute_scaled_gradient_with`]) with the given sample gradient function.
pub fn compute_scaled_gradient<D: Datapoint, G: LossGradient>(
    weights: &Weights,
    batch: Batch<D>,
    eval_scale: ScalingFactor,
) -> Gradient {
    // see above, it should strictly speaking be `/ eval_scale` but `*` is superior
    // because it removes the effect of the eval scale
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
                        G::sample_gradient(wr_prediction, data.outcome()) * data.sampling_weight();
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
            let wr_prediction = wr_prediction_for_weights(weights, data, eval_scale);
            // don't use a separate loop for multiplying with `constant_factor` because the gradient may very well be
            // larger than the numer of samples, so this would likely be slower
            let scaled_delta = constant_factor
                * G::sample_gradient(wr_prediction, data.outcome())
                * data.sampling_weight();
            grad.update(data, scaled_delta);
        }
        grad
    }
}
/// This is where the actual optimization happens.
///
/// Optimize the weights using the given [optimizer](Optimizer) for `num_epochs` epochs, where the gradient is computed
/// over the entire batch each epoch. Regularly prints the current weights using the supplied [weights interpretation](WeightsInterpretation).
pub fn optimize_dataset<D: Datapoint>(
    dataset: &mut Dataset<D>,
    eval_scale: ScalingFactor,
    num_epochs: usize,
    weights_interpretation: &dyn WeightsInterpretation,
    optimizer: &mut dyn Optimizer<D>,
) -> Weights {
    let mut prev_weights: Vec<Weight> = vec![];
    let mut weights = Weights::new(dataset.num_weights());
    let initial_lr_factor = if weights_interpretation.retune_from_zero() {
        0.25
    } else {
        0.5
    };
    optimizer.lr_drop(initial_lr_factor);
    if !weights_interpretation.retune_from_zero() {
        weights = weights_interpretation
            .initial_weights()
            .expect("if `retune_from_zero()` returns `false`, there must be initial weights");
        assert_eq!(
            weights.num_weights(),
            dataset.num_weights(),
            "Incorrect number of initial weights. Maybe your `Eval::NUM_WEIGHTS` is incorrect or your initial_weights() returns incorrect weights?"
        );
    }
    let mut prev_loss = Float::INFINITY;
    let start = Instant::now();
    let print_interval = 50;
    for epoch in 0..num_epochs {
        optimizer.iteration(&mut weights, dataset.as_batch(), eval_scale, epoch);
        if epoch % print_interval == 0 {
            let loss = loss(&weights, dataset.as_batch(), eval_scale);
            println!(
                "Epoch {epoch} complete, weights:\n {}",
                display(weights_interpretation, &weights, &prev_weights)
            );
            let elapsed = start.elapsed();
            // If no weight changed by more than 0.05 within the last `print_interval` epochs, stop.
            let mut max_diff: Float = 0.0;
            for i in 0..prev_weights.len() {
                let diff = weights[i].0 - prev_weights[i].0;
                if diff.abs() > max_diff.abs() {
                    max_diff = diff;
                }
            }
            println!(
                "[{elapsed}s] Epoch {epoch} ({0:.1} epochs/s), quadratic loss: {loss}, loss got smaller by: 1/1_000_000 * {1}, \
                maximum weight change to {print_interval} epochs ago: {max_diff:.2}",
                epoch as f32 / elapsed.as_secs_f32(),
                (prev_loss - loss) * 1_000_000.0,
                elapsed = elapsed.as_secs(),
            );
            if loss <= 0.001 && epoch >= print_interval {
                println!("loss less than epsilon, stopping after {epoch} epochs");
                break;
            }
            if max_diff.abs() <= 0.05 && epoch >= print_interval {
                println!(
                    "Maximum absolute weight change less than 0.05, stopping after {epoch} epochs"
                );
                break;
            }
            prev_weights.clone_from(&weights.0);
            prev_loss = loss;
        }
        if epoch == 20.min(num_epochs / 100) {
            optimizer.lr_drop(1.0 / initial_lr_factor); // undo the raised lr.
        } else if epoch == num_epochs / 2 {
            optimizer.lr_drop(2.0);
        }
    }
    weights
}

/// Convenience function for optimizing with the [`AdamW`] optimizer.
pub fn adamw_optimize<D: Datapoint, G: LossGradient>(
    dataset: &mut Dataset<D>,
    eval_scale: ScalingFactor,
    num_epochs: usize,
    format_weights: &dyn WeightsInterpretation,
) -> Weights {
    let mut optimizer = AdamW::<G>::new(dataset.as_batch(), eval_scale);
    optimize_dataset(
        dataset,
        eval_scale,
        num_epochs,
        format_weights,
        &mut optimizer,
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
        "Scaling factor: {scale:.2}, {0}:\n{1}",
        "Final eval".bold(),
        display(interpretation, &weights, &[])
    );
}

/// The default optimizer. Currently, this is [`Adam`].
pub type DefaultOptimizer = Adam<QuadraticLoss>;

/// Change the current weights each iteration by taking into account the gradient.
///
/// Different implementations mostly differ in their step size control.
pub trait Optimizer<D: Datapoint> {
    /// The gradient of the loss function.
    type Loss: LossGradient
    where
        Self: Sized;

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

    /// A simple but generic optimization procedure. Usually, calling [`optimize_dataset`] (directly or through
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
#[derive(Debug)]
#[must_use]
pub struct SimpleGDOptimizer {
    /// The learning rate.
    pub alpha: Float,
}

impl<D: Datapoint> Optimizer<D> for SimpleGDOptimizer {
    type Loss
        = QuadraticLoss
    where
        Self: Sized;

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
        let gradient =
            compute_scaled_gradient_with(weights, batch, eval_scale, QuadraticLoss::default());
        for i in 0..weights.len() {
            weights[i].0 -= gradient[i].0 * self.alpha;
        }
    }
}

/// Hyperparameters are parameters that control the optimization process and are not themselves
/// automatically optimized.
#[derive(Debug, Copy, Clone)]
pub struct AdamwHyperParams {
    /// Adam Learning rate multiplier, an upper bound on the step size.
    /// This isn't quite the learning rate for [`AdamW`] because it doesn't apply to the weight decay term.
    /// Currently, this implementation does not support a separate learning rate.
    pub alpha: Float,
    /// Exponential decay of the moving average of the gradient
    pub beta1: Float,
    /// Exponential decay of the moving average of the uncentered variance of the gradient
    pub beta2: Float,
    /// Offset to avoid division by zero
    pub epsilon: Float,
    /// Exponential weight decay: Each weight is multiplied by `1 - lambda` each step before the scaled gradient is added.
    /// Using a value of `0` results in the Adam optimizer.
    pub lambda: Float,
}

impl AdamwHyperParams {
    fn for_eval_scale(eval_scale: ScalingFactor) -> Self {
        Self {
            alpha: eval_scale / 40.0,
            // Setting these values too low can introduce crazy swings in the eval values and loss when it would
            // otherwise appear converged -- maybe because of numerical instability?
            beta1: 0.9,
            beta2: 0.999,
            epsilon: 1e-7,
            lambda: 1e-4,
        }
    }
}

/// The default tuner, an implementation of the widely used [Adam](https://arxiv.org/abs/1412.6980) optimizer,
/// which is the same as the [`AdamW`] tuner without weight decay.
#[derive(Debug)]
#[must_use]
pub struct Adam<G: LossGradient>(AdamW<G>);

impl<D: Datapoint, G: LossGradient> Optimizer<D> for Adam<G> {
    type Loss
        = G
    where
        Self: Sized;

    fn new(batch: Batch<D>, eval_scale: ScalingFactor) -> Self
    where
        Self: Sized,
    {
        Self(AdamW::adam(batch, eval_scale))
    }

    fn lr_drop(&mut self, factor: Float) {
        <AdamW<G> as Optimizer<D>>::lr_drop(&mut self.0, factor);
    }

    fn iteration(
        &mut self,
        weights: &mut Weights,
        batch: Batch<'_, D>,
        eval_scale: ScalingFactor,
        i: usize,
    ) {
        self.0.iteration(weights, batch, eval_scale, i);
    }
}

/// An implementation of the very widely used [AdamW](https://arxiv.org/abs/1711.05101) optimizer,
/// which extends the [`Adam`] optimizer with weight decay.
#[derive(Debug)]
#[must_use]
pub struct AdamW<G: LossGradient> {
    /// Hyperparameters. Should be set before starting to optimize.
    pub hyper_params: AdamwHyperParams,
    /// first moment (exponentially moving average)
    m: Weights,
    /// second moment (exponentially moving average)
    v: Weights,
    _phantom: PhantomData<G>,
}

impl<G: LossGradient> AdamW<G> {
    /// Create a new `Adam` optimizer, which is the same as an [`AdamW`] optimizer with the `lambda` hyperparameter
    /// set to zero.
    pub fn adam<D: Datapoint>(batch: Batch<D>, eval_scale: ScalingFactor) -> Self {
        let mut res = Self::new(batch, eval_scale);
        res.hyper_params.lambda = 0.0;
        res
    }
}

impl<D: Datapoint, G: LossGradient> Optimizer<D> for AdamW<G> {
    type Loss
        = G
    where
        Self: Sized;

    fn new(batch: Batch<D>, eval_scale: ScalingFactor) -> Self {
        let hyper_params = AdamwHyperParams::for_eval_scale(eval_scale);
        Self {
            hyper_params,
            m: Weights::new(batch.num_weights),
            v: Weights::new(batch.num_weights),
            _phantom: PhantomData,
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
        let gradient = compute_scaled_gradient::<D, G>(weights, batch, eval_scale);
        for i in 0..gradient.len() {
            // biased since the values are initialized to 0, so the exponential moving average is wrong
            self.m[i] = self.m[i] * beta1 + gradient[i] * (1.0 - beta1);
            self.v[i] = self.v[i] * beta2 + gradient[i] * gradient[i].0 * (1.0 - beta2);
            let unbiased_m = self.m[i] / (1.0 - beta1.powi(iteration as i32));
            let unbiased_v = self.v[i] / (1.0 - beta2.powi(iteration as i32));
            let w = weights[i];
            weights[i] -= w * self.hyper_params.lambda
                + unbiased_m * self.hyper_params.alpha
                    / (unbiased_v.0.sqrt() + self.hyper_params.epsilon);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::distributions::{Distribution, Uniform};
    use rand::thread_rng;
    use std::cmp::Ordering;
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
            for eval_scale in 1..100_i8 {
                let loss = loss_for(
                    &weights,
                    batch,
                    ScalingFactor::from(eval_scale),
                    quadratic_sample_loss,
                );
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
        for outcome in [0.0, 0.5, 1.0] {
            let data_points = [NonTaperedDatapoint {
                features: vec![Feature::new(1, 0)],
                outcome: Outcome::new(outcome),
            }];
            let batch = Batch {
                datapoints: data_points.as_slice(),
                num_weights: 1,
                weight_sum: 1.0,
            };
            let gradient = compute_scaled_gradient::<NonTaperedDatapoint, CrossEntropyLoss>(
                &weights, batch, 1.0,
            );
            assert_eq!(gradient.len(), 1);
            let gradient_value = gradient[0].0;
            let sgn = |x| {
                if x > 0.0 {
                    1.0
                } else if x < 0.0 {
                    -1.0
                } else {
                    0.0
                }
            };
            // assert_eq!(-gradient, 0.5 * 0.5 * 0.5 * 2.0 * outcome.signum());
            assert_eq!(-gradient_value, sgn(outcome - 0.5), "{outcome}");
        }
    }

    #[test]
    // testcase that contains only 1 position with only 1 feature
    pub fn trivial_test() {
        let scaling_factor = 42.0;
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
                        let grad = compute_scaled_gradient_with(
                            &weights,
                            batch,
                            scaling_factor,
                            QuadraticLoss::default(),
                        );
                        let old_weights = weights.clone();
                        weights -= &(grad.clone() * 0.5);
                        // println!("loss {0}, initial weight {initial_weight}, weights {weights}, gradient {grad}, eval {1}, predicted {2}, outcome {outcome}, feature {feature}, scaling factor {scaling_factor}", loss(&weights, &dataset, scaling_factor), cp_eval_for_weights(&weights, &dataset[0].position), wr_prediction_for_weights(&weights, &dataset[0].position, scaling_factor));
                        if initial_weight == 0.0 && grad.0[0].0.abs() > 0.000_000_1 {
                            assert_eq!(
                                weights.0[0].0.partial_cmp(&old_weights[0].0),
                                outcome.partial_cmp(&0.5).map(|x| match feature.cmp(&0) {
                                    Ordering::Less => x.reverse(),
                                    Equal => Equal,
                                    Ordering::Greater => x,
                                })
                            );
                        }
                        let new_loss = loss(&weights, batch, scaling_factor);
                        let old_loss = loss(&old_weights, batch, scaling_factor);
                        assert!(new_loss >= 0.0, "{new_loss}");
                        assert!(new_loss - old_loss <= 1e-10, "new loss: {new_loss}, old loss: {old_loss}, feature {feature}, initial weight {initial_weight}, outcome {outcome}");
                    }
                    let loss = loss_for(&weights, batch, scaling_factor, quadratic_sample_loss);
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
            for i in 0..100 {
                let grad =
                    compute_scaled_gradient_with(&weights, batch, 1.0, QuadraticLoss::default());
                let old_weights = weights.clone();
                weights -= &grad;
                let new_loss = loss(&weights, batch, 1.0);
                let old_loss = loss(&old_weights, batch, 1.0);
                assert!(new_loss - old_loss <= 1e-10, "{i}: {new_loss} {old_loss}");
            }
            let loss = loss_for(&weights, batch, 1.0, quadratic_sample_loss);
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
            let mut lr = 0.2;
            for i in 0..100 {
                let grad = compute_scaled_gradient_with(
                    &weights,
                    batch,
                    scale,
                    CrossEntropyLoss::default(),
                );
                let old_weights = weights.clone();
                weights -= &(grad.clone() * lr);
                let current_loss = loss(&weights, batch, scale);
                let old_loss = loss(&old_weights, batch, scale);
                assert!(current_loss <= old_loss, "{i} {current_loss} {old_loss}");
                lr *= 0.9;
            }
            let loss = loss_for(&weights, batch, scale, quadratic_sample_loss);
            assert!(loss <= 0.01, "{loss}");
            if outcome == 0.5 {
                assert_eq!(weights[0].0.signum(), weights[1].0.signum());
                assert!((weights[0].0.abs() - weights[1].0.abs()).abs() <= 0.0000_0001);
            } else {
                assert_eq!(weights[0].0 > weights[1].0, outcome > 0.5);
            }
        }
    }

    #[test]
    pub fn two_positions_test() {
        type AnyOptimizer = Box<dyn Optimizer<NonTaperedDatapoint>>;
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
                let grad = compute_scaled_gradient_with(
                    &weights,
                    batch,
                    scale,
                    CrossEntropyLoss::default(),
                );
                weights -= &grad;
            }
            let remaining_loss = loss_for(&weights, batch, scale, quadratic_sample_loss);
            assert!(remaining_loss <= 0.001);
            assert!(weights[0].0 >= 100.0);
            assert!(weights[1].0 <= -100.0);

            let optimizers: [AnyOptimizer; 5] = [
                Box::new(SimpleGDOptimizer { alpha: 1.0 }),
                Box::new(Adam::<QuadraticLoss>::new(batch, scale)),
                Box::new(Adam::<CrossEntropyLoss>::new(batch, scale)),
                Box::new(AdamW::<QuadraticLoss>::new(batch, scale)),
                Box::new(AdamW::<CrossEntropyLoss>::new(batch, scale)),
            ];
            for mut optimizer in optimizers {
                for i in 0..200 {
                    optimizer.iteration(&mut weights_copy, batch, scale, i);
                }
                let remaining_loss = loss_for(&weights_copy, batch, scale, quadratic_sample_loss);
                assert!(remaining_loss <= 0.001, "{remaining_loss}");
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
            let grad = compute_scaled_gradient_with(&weights, batch, 1.0, QuadraticLoss::default());
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
            let mut adam = Adam::<QuadraticLoss>::new(batch, eval_scale);
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
