use crate::eval::{Eval, WeightFormatter};
use derive_more::{Add, AddAssign, Deref, DerefMut, Display, Div, Mul, Sub, SubAssign};
use gears::games::Color;
use rand::prelude::SliceRandom;
use rand::thread_rng;
use rayon::prelude::*;
use std::fmt::{Debug, Formatter};
use std::iter::Sum;
use std::ops::{DivAssign, MulAssign};
use std::time::Instant;
use std::usize;

// TODO: Better value
/// Not doing multithreading for small batch sizes isn't only meant to improve performance,
/// it also makes it easier to debug problems with the eval because stack traces and debugger steps
/// are simpler.
const MIN_MULTITHREADING_BATCH_SIZE: usize = 10_000;

pub type Float = f64;

/// The result of calling the eval function.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub struct CpScore(pub Float);

impl Display for CpScore {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}cp", self.0)
    }
}

/// The wr prediction, based on the CpScore (between 0 and 1).
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
pub struct WrScore(pub Float);

/// `WrScore` is used for the converted score returned by the eval, `Outcome` for the actual outcome
pub type Outcome = WrScore;

impl WrScore {
    pub fn new(val: Float) -> Self {
        assert!(val >= 0.0 && val <= 1.0);
        Self(val)
    }
}

impl Display for WrScore {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.3}", self.0)
    }
}

/// The eval scale stretches the sigmoid horizontally, so a larger eval scale means that
/// a larger eval value is necessary to count as "surely lost/won". It determines how to convert a `CpScore` to a `WrScore`.
pub type ScalingFactor = Float;

pub fn sigmoid(x: Float, scale: ScalingFactor) -> Float {
    1.0 / (1.0 + (-x / scale).exp())
}

pub fn cp_to_wr(cp: CpScore, eval_scale: ScalingFactor) -> WrScore {
    WrScore(sigmoid(cp.0, eval_scale))
}

pub fn sample_loss(wr_prediction: WrScore, outcome: Outcome) -> Float {
    let delta = wr_prediction.0 - outcome.0;
    delta * delta
}

pub fn sample_loss_for_cp(eval: CpScore, outcome: Outcome, eval_scale: ScalingFactor) -> Float {
    let wr_prediction = cp_to_wr(eval, eval_scale);
    sample_loss(wr_prediction, outcome)
}

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
    pub fn rounded(self) -> i32 {
        self.0.round() as i32
    }
}

/// In an ideal world, this would take the number N of weights as a generic parameter.
/// However, const generics are very limited in (stable) Rust, which makes this a pain to implement.
/// So instead, the size is known runtime.

#[derive(Debug, Default, Clone, Deref, DerefMut)]
pub struct Weights(pub Vec<Weight>);

pub type Gradient = Weights;

impl Weights {
    pub fn new(num_weights: usize) -> Self {
        Self(vec![Weight(0.0); num_weights])
    }
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

#[derive(Debug, Default, Copy, Clone, PartialOrd, PartialEq)]
pub struct Feature {
    feature: FeatureT,
    idx: u16,
}

impl Feature {
    pub fn new(feature: FeatureT, idx: u16) -> Self {
        Self { feature, idx }
    }

    pub fn float(self) -> Float {
        self.feature as Float
    }
    pub fn idx(self) -> usize {
        self.idx as usize
    }
}

#[derive(Debug, Copy, Clone)]
pub struct PhaseMultiplier(Float);

pub trait TraceTrait: Debug + Default {
    fn as_features(&self, idx_offset: usize) -> Vec<Feature>;
    fn phase(&self) -> Float {
        1.0
    }
}

#[derive(Debug, Default)]
pub struct SimpleTrace {
    pub white: Vec<isize>,
    pub black: Vec<isize>,
    pub phase: Float,
}

impl SimpleTrace {
    pub fn for_features(num_features: usize) -> Self {
        Self {
            white: vec![0; num_features],
            black: vec![0; num_features],
            phase: 0.0,
        }
    }
    pub fn increment(&mut self, idx: usize, color: Color) {
        self.increment_by(idx, color, 1);
    }

    pub fn increment_by(&mut self, idx: usize, color: Color, amount: isize) {
        match color {
            Color::White => self.white[idx] += amount,
            Color::Black => self.black[idx] += amount,
        };
    }
}

impl TraceTrait for SimpleTrace {
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
        res.sort_by(|a, b| a.idx().cmp(&b.idx()));
        res
    }

    fn phase(&self) -> Float {
        self.phase
    }
}

struct WeightedFeature {
    weight: Float,
    idx: usize,
}

impl WeightedFeature {
    fn new(idx: usize, weight: Float) -> Self {
        Self { weight, idx }
    }
}

pub trait Datapoint: Clone + Send + Sync {
    fn new<T: TraceTrait>(trace: T, outcome: Outcome) -> Self;
    fn outcome(&self) -> Outcome;
    fn features(&self) -> impl Iterator<Item = WeightedFeature>;
}

#[derive(Debug, Clone)]
pub struct NonTaperedDatapoint {
    pub features: Vec<Feature>,
    pub outcome: Outcome,
}

impl Datapoint for NonTaperedDatapoint {
    fn new<T: TraceTrait>(trace: T, outcome: Outcome) -> Self {
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

#[derive(Debug, Clone)]
pub struct TaperedDatapoint {
    pub features: Vec<Feature>,
    pub outcome: Outcome,
    pub phase: PhaseMultiplier,
}

impl Datapoint for TaperedDatapoint {
    fn new<T: TraceTrait>(trace: T, outcome: Outcome) -> Self {
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

// TODO: Let `Dataset` own the list of features and `D` objects only hold a slice of features
#[derive(Debug)]
pub struct Dataset<D: Datapoint> {
    pub datapoints: Vec<D>,
    pub num_weights: usize,
}

impl<D: Datapoint> Dataset<D> {
    pub fn new(num_weights: usize) -> Self {
        Self {
            datapoints: vec![],
            num_weights,
        }
    }

    pub fn union(&mut self, mut other: Dataset<D>) {
        assert_eq!(self.num_weights, other.num_weights);
        self.datapoints.append(&mut other.datapoints);
    }

    pub fn shuffle(&mut self) {
        self.datapoints.shuffle(&mut thread_rng());
    }

    pub fn as_batch(&self) -> Batch<D> {
        Batch {
            datapoints: &self.datapoints,
            num_weights: self.num_weights,
        }
    }
    pub fn batch(&self, start_idx: usize, end_idx: usize) -> Batch<D> {
        Batch {
            datapoints: &self.datapoints[start_idx..end_idx],
            num_weights: self.num_weights,
        }
    }
}

#[derive(Debug)]
pub struct Batch<'a, D: Datapoint> {
    pub datapoints: &'a [D],
    pub num_weights: usize,
}

// deriving Copy, Clone doesn't work for some reason
impl<D: Datapoint> Clone for Batch<'_, D> {
    fn clone(&self) -> Self {
        Self {
            datapoints: self.datapoints,
            num_weights: self.num_weights,
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

pub fn cp_eval_for_weights<D: Datapoint>(weights: &Weights, position: &D) -> CpScore {
    let mut res = 0.0;
    for feature in position.features() {
        res += feature.weight * weights[feature.idx].0;
    }
    CpScore(res)
}

pub fn wr_prediction_for_weights<D: Datapoint>(
    weights: &Weights,
    position: &D,
    eval_scale: ScalingFactor,
) -> WrScore {
    let eval = cp_eval_for_weights(weights, position);
    cp_to_wr(eval, eval_scale)
}

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
                let loss = sample_loss(eval, datapoint.outcome());
                debug_assert!(loss >= 0.0);
                loss
            })
            .sum()
    } else {
        let mut res = Float::default();
        for datapoint in batch.iter() {
            let eval = wr_prediction_for_weights(weights, datapoint, eval_scale);
            let loss = sample_loss(eval, datapoint.outcome());
            debug_assert!(loss >= 0.0);
            res += loss;
        }
        res
    };
    sum / batch.len() as Float
}

/// Computes the gradient of the loss function:
/// The loss function of a single sample is `(sigmoid(sample, scale) - outcome) ^ 2`,
/// so per the chain rule, the derivative is `2 * (sigmoid(sample, scale) - outcome) * sigmoid'(sample, scale)`,
/// where the derivative of the sigmoid, sigmoid', is `scale * sigmoid(sample, scale) * (1 - sigmoid(sample, scale)`.
pub fn compute_gradient<D: Datapoint>(
    weights: &Weights,
    batch: Batch<D>,
    eval_scale: ScalingFactor,
) -> Gradient {
    // the 2 is a constant factor and could be dropped because we don't need to preserve the magnitude of the gradient,
    // but let's be correct and keep it.
    let constant_factor = 2.0 * eval_scale / batch.len() as f64;
    if batch.len() >= MIN_MULTITHREADING_BATCH_SIZE {
        batch
            .datapoints
            .par_iter()
            .fold(
                || Gradient::new(weights.num_weights()),
                |mut grad: Gradient, data: &D| {
                    let wr_prediction = wr_prediction_for_weights(weights, data, eval_scale).0;

                    // constant factors have been moved outside the loop
                    let scaled_delta =
                        (wr_prediction - data.outcome().0) * wr_prediction * (1.0 - wr_prediction);
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
                * (1.0 - wr_prediction);
            grad.update(data, scaled_delta);
        }
        grad
    }
}

pub fn optimize_entire_batch<D: Datapoint>(
    batch: Batch<D>,
    eval_scale: ScalingFactor,
    num_epochs: usize,
    format_weights: &dyn WeightFormatter,
    optimizer: &mut dyn Optimizer<D>,
) -> Weights {
    let mut prev_weights: Vec<Weight> = vec![];
    let mut weights = Weights::new(batch.num_weights);
    // Since weights are initially 0, use a very high lr for the first couple of iterations.
    optimizer.lr_drop(0.25); // increases lr by a factor of
    let mut prev_loss = Float::INFINITY;
    let start = Instant::now();
    for epoch in 0..num_epochs {
        optimizer.iteration(&mut weights, batch, eval_scale, epoch);
        if epoch % 50 == 0 {
            let loss = loss(&weights, batch, eval_scale);
            println!(
                "Epoch {epoch} complete, weights:\n {}",
                format_weights.display(&weights)
            );
            let elapsed = start.elapsed();
            // If no weight changed by more than 0.1 within the last 50 epochs, stop.
            let max_diff = prev_weights
                .iter()
                .zip(weights.0.iter())
                .map(|(a, b)| ((a.0 - b.0).abs() * 100.0).round() as u64)
                .max()
                .unwrap_or(u64::MAX);
            println!(
                "[{elapsed}s] Epoch {epoch} ({0:.1} epochs/s), loss: {loss}, loss got smaller by: 1/1_000_000 * {1}, \
                maximum weight change in: {2:.3}",
                epoch as f32 / elapsed.as_secs_f32(),
                (prev_loss - loss) * 1_000_000.0,
                max_diff as Float / 100.0,
                elapsed = elapsed.as_secs()
            );
            if loss <= 0.001 && epoch >= 20 {
                break;
            }
            if max_diff <= 5 && epoch >= 50 {
                break;
            }
            prev_weights = weights.0.clone();
            prev_loss = loss;
        }
        if epoch == 20.min(num_epochs / 100) {
            optimizer.lr_drop(4.0); // undo the raised lr.
        } else if epoch == num_epochs / 2 {
            optimizer.lr_drop(1.5);
        }
    }
    weights
}

fn adam_optimize<D: Datapoint>(
    batch: Batch<D>,
    eval_scale: ScalingFactor,
    num_epochs: usize,
    format_weights: &dyn WeightFormatter,
) -> Weights {
    optimize_entire_batch(
        batch,
        eval_scale,
        num_epochs,
        format_weights,
        &mut Adam::new(batch, eval_scale),
    )
}

pub trait Optimizer<D: Datapoint> {
    fn new(batch: Batch<D>, eval_scale: ScalingFactor) -> Self
    where
        Self: Sized;

    // can be less than 1 to increase the lr.
    fn lr_drop(&mut self, factor: Float);

    fn iteration(
        &mut self,
        weights: &mut Weights,
        batch: Batch<'_, D>,
        eval_scale: ScalingFactor,
        i: usize,
    );

    /// A simple but generic optimization procedure. Usually, calling `do_optimize` results (directly or through
    /// the `optimize` function in `lib.rs`) results in faster convergence. This function is primarily useful for debugging.
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

pub struct SimpleGDOptimizer {
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

// impl Default for AdamHyperParams {
//     fn default() -> Self {
//         Self::for_eval_scale(100.0)
//     }
// }

impl AdamHyperParams {
    fn for_eval_scale(eval_scale: ScalingFactor) -> Self {
        Self {
            alpha: eval_scale / 10.0,
            /// Setting these values too low can introduce crazy swings in the eval values and loss when it would
            /// otherwise appear converged -- maybe because of numerical instability?
            beta1: 0.9, // 0.8,
            beta2: 0.999,
            /// When the gradient goes down to zero, we can run into numerical instability issues when dividing by the
            /// square root of the uncentered variance, Set epsilon relatively large to counter this effect. This can
            /// happen when a weight has almost converged or (if something else went wrong) when a weight got tuned so
            /// large that the sigmoid gradient vanishes.
            epsilon: 1e-8,
        }
    }
}

#[derive(Debug)]
pub struct Adam {
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
            weights[i] = weights[i]
                - unbiased_m * self.hyper_params.alpha
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
                num_weights: 2,
            };
            for eval_scale in 1..100 {
                let loss = loss(&weights, batch, eval_scale as ScalingFactor);
                if outcome == 0.5 {
                    assert_eq!(loss, 0.0);
                } else {
                    assert!((loss - 0.25).abs() <= 0.0001);
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
                    dataset.datapoints.push(datapoint);
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
                        assert!(
                            loss(&weights, batch, scaling_factor)
                                <= loss(&old_weights, batch, scaling_factor)
                        );
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
                num_weights: 2,
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
            assert!(loss <= 0.01);
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
            let mut optimizers: [AnyOptimizer; 2] = [
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
            assert!(loss(&weights, batch, 1.0) <= loss(&weights, batch, 1.0));
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
