use crate::eval::{Eval, FormatWeights};
use derive_more::{Add, AddAssign, Deref, DerefMut, Display, Div, Mul, Sub, SubAssign};
use std::fmt::Formatter;
use std::ops::{DivAssign, MulAssign};

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
pub struct WrScore(Float);

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
/// a larger eval value is necessary to count as "surely lost/won". It determines how to convert a `CpScore` to a `WrScore`
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq, Display)]
pub struct EvalScale(pub Float);

impl Default for EvalScale {
    fn default() -> Self {
        Self(110.0) // gives roughly the normal piece values, expressed as centipawns
    }
}

pub fn sigmoid(x: Float, scale: EvalScale) -> Float {
    1.0 / (1.0 + (-x / scale.0).exp())
}

pub fn cp_to_wr(cp: CpScore, eval_scale: EvalScale) -> WrScore {
    WrScore(sigmoid(cp.0 as Float, eval_scale))
}

pub fn sample_delta(wr_prediction: WrScore, outcome: Outcome) -> Float {
    wr_prediction.0 - outcome.0
}

pub fn sample_loss(wr_prediction: WrScore, outcome: Outcome) -> Float {
    let delta = sample_delta(wr_prediction, outcome);
    delta * delta
}

pub fn sample_loss_for_cp(eval: CpScore, outcome: Outcome, eval_scale: EvalScale) -> Float {
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

#[derive(Debug, Default, Clone, Deref, DerefMut)]
pub struct Weights(Vec<Weight>);

pub type Gradient = Weights;

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

impl AddAssign for Weights {
    fn add_assign(&mut self, rhs: Self) {
        self.iter_mut().zip(rhs.iter()).for_each(|(a, b)| *a += *b)
    }
}

impl SubAssign for Weights {
    fn sub_assign(&mut self, rhs: Self) {
        self.iter_mut().zip(rhs.iter()).for_each(|(a, b)| *a -= *b)
    }
}

impl Add for Weights {
    type Output = Weights;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl Sub for Weights {
    type Output = Weights;

    fn sub(mut self, rhs: Self) -> Self::Output {
        self -= rhs;
        self
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
    fn update(&mut self, other: &Position, factor: Float) {
        for (w, rhs) in self.iter_mut().zip(other.iter()) {
            w.0 += rhs.0 as Float * factor;
        }
    }
}

#[cfg(not(feature = "tapered"))]
type FeatureImpl = i8;

#[cfg(feature = "tapered")]
pub type FeatureT = Float;

#[derive(Debug, Default, Copy, Clone, PartialOrd, PartialEq)]
pub struct Feature(pub FeatureT);

impl Feature {
    pub fn new_i8(val: i8) -> Self {
        Self(val as FeatureT)
    }

    #[cfg(feature = "tapered")]
    pub fn new_float(val: Float) -> Self {
        Self(val as FeatureT)
    }

    pub fn float(self) -> Float {
        self.0 as Float
    }
}

pub type Position = Vec<Feature>;

pub struct Datapoint {
    pub position: Position,
    pub outcome: Outcome,
}

pub type Dataset = Vec<Datapoint>;

pub fn cp_eval_for_weights(weights: &Weights, position: &Position) -> CpScore {
    debug_assert_eq!(weights.len(), position.len());
    CpScore(
        weights
            .iter()
            .zip(position.iter())
            .map(|(w, feature)| w.0 * feature.0 as Float)
            .sum::<Float>(),
    )
}

pub fn wr_prediction_for_weights(
    weights: &Weights,
    position: &Position,
    eval_scale: EvalScale,
) -> WrScore {
    let eval = cp_eval_for_weights(weights, position);
    cp_to_wr(eval, eval_scale)
}

pub fn loss(weights: &Weights, dataset: &[Datapoint], eval_scale: EvalScale) -> Float {
    let mut res = Float::default();
    for datapoint in dataset.iter() {
        let eval = wr_prediction_for_weights(weights, &datapoint.position, eval_scale);
        let loss = sample_loss(eval, datapoint.outcome);
        debug_assert!(loss >= 0.0);
        res += loss;
    }
    res / dataset.len() as Float
}

/// Computes the gradient of the loss function:
/// The loss function of a single sample is `(sigmoid(sample, scale) - outcome) ^ 2`,
/// so per the chain rule, the derivative is `2 * (sigmoid(sample, scale) - outcome) * sigmoid'(sample, scale)`,
/// where the derivative of the sigmoid, sigmoid', is `scale * sigmoid(sample, scale) * (1 - sigmoid(sample, scale)`.
pub fn compute_gradient(
    weights: &Weights,
    dataset: &[Datapoint],
    eval_scale: EvalScale,
) -> Gradient {
    let mut grad = Weights(vec![Weight::default(); weights.len()]);
    // the 2 is a constant factor and could be dropped because we don't need to preserve the magnitude of the gradient,
    // but let's be correct and keep it.
    let constant_factor = 2.0 * eval_scale.0 / dataset.len() as f64;
    for data in dataset.iter() {
        let wr_prediction = wr_prediction_for_weights(weights, &data.position, eval_scale).0;

        // constant factors have been moved outside the loop
        let scaled_delta = constant_factor
            * (wr_prediction - data.outcome.0)
            * wr_prediction
            * (1.0 - wr_prediction);
        grad.update(&data.position, scaled_delta);
    }
    grad
}

pub trait Optimizer {
    fn new(dataset: &[Datapoint], eval_scale: EvalScale) -> Self
    where
        Self: Sized;

    // can be less than 1 to increase the lr.
    fn lr_drop(&mut self, factor: Float);

    fn num_features(&self) -> Option<usize> {
        None
    }

    fn iteration(
        &mut self,
        weights: &mut Weights,
        dataset: &[Datapoint],
        eval_scale: EvalScale,
        i: usize,
    );

    fn optimize(
        &mut self,
        dataset: &[Datapoint],
        eval_scale: EvalScale,
        num_iterations: usize,
    ) -> Weights {
        if let Some(num_features) = self.num_features() {
            assert_eq!(
                dataset[0].position.len(),
                num_features,
                "The number of features in the dataset doesn't match"
            );
        }
        let mut weights = Weights(vec![Weight(0.0); dataset[0].position.len()]);
        for i in 0..num_iterations {
            self.iteration(&mut weights, dataset, eval_scale, i);
        }
        weights
    }
}

pub struct SimpleGDOptimizer {
    pub alpha: Float,
}

impl Optimizer for SimpleGDOptimizer {
    fn new(_dataset: &[Datapoint], eval_scale: EvalScale) -> Self {
        Self {
            alpha: eval_scale.0 / 4.0,
        }
    }

    fn lr_drop(&mut self, factor: Float) {
        self.alpha /= factor;
    }

    fn iteration(
        &mut self,
        weights: &mut Weights,
        dataset: &[Datapoint],
        eval_scale: EvalScale,
        _i: usize,
    ) {
        let gradient = compute_gradient(weights, dataset, eval_scale);
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

impl Default for AdamHyperParams {
    fn default() -> Self {
        Self::for_eval_scale(EvalScale::default())
    }
}

impl AdamHyperParams {
    fn for_eval_scale(eval_scale: EvalScale) -> Self {
        Self {
            /// We can use relatively large step sizes since our objective function is pretty nice
            alpha: eval_scale.0 / 4.0,
            /// Setting these values too low can introduce crazy swings in the eval values and loss when it would
            /// otherwise appear converged -- maybe because of numerical instability?
            beta1: 0.9, // 0.8,
            beta2: 0.995,
            /// When the gradient goes down to zero, we can run into numerical instability issues when dividing by the
            /// square root of the uncentered variance, Set epsilon relatively large to counter this effect. This can
            /// happen when a weight has almost converged or (if something else went wrong) when a weight got tuned so
            /// large that the sigmoid gradient vanishes.
            epsilon: 1e-6,
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

impl Optimizer for Adam {
    fn new(dataset: &[Datapoint], eval_scale: EvalScale) -> Self {
        let hyper_params = AdamHyperParams::for_eval_scale(eval_scale);
        let num_weights = dataset[0].position.len();
        Self {
            hyper_params,
            m: Weights(vec![Weight(0.0); num_weights]),
            v: Weights(vec![Weight(0.0); num_weights]),
        }
    }

    fn lr_drop(&mut self, factor: Float) {
        self.hyper_params.alpha /= factor;
    }

    fn num_features(&self) -> Option<usize> {
        Some(self.v.len())
    }

    fn iteration(
        &mut self,
        weights: &mut Weights,
        dataset: &[Datapoint],
        eval_scale: EvalScale,
        iteration: usize,
    ) {
        let iteration = iteration + 1;
        let beta1 = self.hyper_params.beta1;
        let beta2 = self.hyper_params.beta2;
        let gradient = compute_gradient(weights, dataset, eval_scale);
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

pub fn do_optimize(
    dataset: &[Datapoint],
    eval_scale: EvalScale,
    num_epochs: usize,
    mut format_weights: FormatWeights,
    optimizer: &mut dyn Optimizer,
) -> Weights {
    // TODO: AdamW?
    let mut weights = Weights(vec![Weight(0.0); dataset[0].position.len()]);
    // Since weights are initially 0, use a very high lr for the first couple of iterations.
    optimizer.lr_drop(0.25); // increases lr by a factor of
    let mut prev_loss = 0.0;
    for epoch in 0..num_epochs {
        optimizer.iteration(&mut weights, dataset, eval_scale, epoch);
        let loss = loss(&weights, dataset, eval_scale);
        println!(
            "Epoch {epoch} complete, weights:\n {}",
            format_weights.with_weights(weights.clone())
        );
        // if epoch % 10 == 0 {
        // let loss = loss(&weights, dataset, eval_scale);
        println!(
            "Epoch {epoch}, loss: {0}, loss got smaller by: 1/1_000_000 * {1}",
            loss,
            (prev_loss - loss) * 1_000_000.0,
        );
        prev_loss = loss;
        if loss <= 0.01 && epoch >= 20 {
            break;
        }
        // }
        if epoch == 20.min(num_epochs / 100) {
            optimizer.lr_drop(4.0); // undo the raised lr.
        }
    }
    weights
}

pub fn adam_optimize(
    dataset: &[Datapoint],
    eval_scale: EvalScale,
    num_epochs: usize,
    format_weights: FormatWeights,
) -> Weights {
    do_optimize(
        dataset,
        eval_scale,
        num_epochs,
        format_weights,
        &mut Adam::new(dataset, eval_scale),
    )
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
            let dataset = vec![Datapoint {
                position: no_features.clone(),
                outcome: Outcome::new(outcome),
            }];
            for eval_scale in 1..100 {
                let loss = loss(&weights, &dataset, EvalScale(eval_scale as Float));
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
            let data_points = [Datapoint {
                position: vec![Feature::new_i8(1)],
                outcome: Outcome::new(1.0),
            }];
            let gradient = compute_gradient(&weights, &data_points, EvalScale(1.0));
            assert_eq!(gradient.len(), 1);
            let gradient = gradient[0].0;
            assert_eq!(-gradient, 0.5 * 0.5 * 0.5 * 2.0 * outcome.signum());
        }
    }

    #[test]
    // testcase that contains only 1 position with only 1 feature
    pub fn trivial_test() {
        let scaling_factor = EvalScale(100.0);
        for feature in [1, 2, -1, 0] {
            for initial_weight in [0.0, 0.1, 100.0, -1.2] {
                for outcome in [0.0, 0.5, 1.0, 0.9, 0.499] {
                    let mut weights = Weights(vec![Weight(initial_weight)]);
                    let position = vec![Feature::new_i8(feature)];
                    let datapoint = Datapoint {
                        position: position.clone(),
                        outcome: Outcome::new(outcome),
                    };
                    let dataset = vec![datapoint];
                    for _ in 0..100 {
                        let grad = compute_gradient(&weights, &dataset, scaling_factor);
                        let old_weights = weights.clone();
                        weights -= grad.clone();
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
                            loss(&weights, &dataset, scaling_factor)
                                <= loss(&old_weights, &dataset, scaling_factor)
                        );
                    }
                    let loss = loss(&weights, &dataset, scaling_factor);
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
            let position = vec![Feature::new_i8(1), Feature::new_i8(-1), Feature::new_i8(2)];
            let datapoint = Datapoint {
                position: position.clone(),
                outcome: Outcome::new(outcome),
            };
            let dataset = vec![datapoint];
            for _ in 0..100 {
                let grad = compute_gradient(&weights, &dataset, EvalScale(1.0));
                let old_weights = weights.clone();
                weights -= grad.clone();
                assert!(
                    loss(&weights, &dataset, EvalScale(1.0))
                        <= loss(&old_weights, &dataset, EvalScale(1.0))
                );
            }
            let loss = loss(&weights, &dataset, EvalScale(1.0));
            assert!(loss <= 0.01);
        }
    }

    #[test]
    pub fn two_features_test() {
        let scale = EvalScale(500.0);
        for outcome in [0.0, 0.5, 1.0] {
            let mut weights = Weights(vec![Weight(123.987), Weight(-987.123)]);
            let position = vec![Feature::new_i8(3), Feature::new_i8(-3)];
            let datapoint = Datapoint {
                position: position.clone(),
                outcome: Outcome::new(outcome),
            };
            let dataset = vec![datapoint];
            let mut lr = 1.0;
            for _ in 0..100 {
                let grad = compute_gradient(&weights, &dataset, scale);
                let old_weights = weights.clone();
                weights -= grad.clone() * lr;
                let current_loss = loss(&weights, &dataset, scale);
                let old_loss = loss(&old_weights, &dataset, scale);
                assert!(current_loss <= old_loss);
                lr *= 0.99;
            }
            let loss = loss(&weights, &dataset, scale);
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
        let scale = EvalScale(1000.0);
        let win = Datapoint {
            position: vec![Feature::new_i8(1), Feature::new_i8(-1)],
            outcome: WrScore(1.0),
        };
        let lose = Datapoint {
            position: vec![Feature::new_i8(-1), Feature::new_i8(1)],
            outcome: WrScore(0.0),
        };
        let dataset = vec![win, lose];
        let weights_dist = Uniform::new(-100.0, 100.0);
        let mut rng = thread_rng();
        for _ in 0..100 {
            let mut weights = Weights(vec![
                Weight(weights_dist.sample(&mut rng)),
                Weight(weights_dist.sample(&mut rng)),
            ]);
            let mut weights_copy = weights.clone();
            for _ in 0..200 {
                let grad = compute_gradient(&weights, &dataset, scale);
                weights -= grad;
            }
            let remaining_loss = loss(&weights, &dataset, scale);
            assert!(remaining_loss <= 0.001);
            assert!(weights[0].0 >= 100.0);
            assert!(weights[1].0 <= -100.0);

            type AnyOptimizer = Box<dyn Optimizer>;
            let mut optimizers: [Box<dyn Optimizer>; 2] = [
                Box::new(SimpleGDOptimizer { alpha: 1.0 }),
                Box::new(Adam::new(&dataset, scale)),
            ];
            for mut optimizer in optimizers {
                for i in 0..100 {
                    optimizer.iteration(&mut weights_copy, &dataset, scale, i);
                }
                let remaining_loss = loss(&weights_copy, &dataset, scale);
                assert!(remaining_loss <= 0.001);
                assert!(weights[0].0 >= 100.0);
                assert!(weights[1].0 <= -100.0);
            }
        }
    }

    #[test]
    pub fn three_positions_test() {
        let mut weights = Weights(vec![Weight(0.4), Weight(1.0), Weight(2.0)]);
        let draw_datapoint = Datapoint {
            position: vec![Feature::new_i8(0), Feature::new_i8(0), Feature::new_i8(0)],
            outcome: Outcome::new(0.5),
        };
        let lose_datapoint = Datapoint {
            position: vec![Feature::new_i8(-1), Feature::new_i8(-1), Feature::new_i8(0)],
            outcome: Outcome::new(0.0),
        };
        let win_datapoint = Datapoint {
            position: vec![Feature::new_i8(1), Feature::new_i8(1), Feature::new_i8(0)],
            outcome: Outcome::new(1.0),
        };

        let dataset = vec![draw_datapoint, win_datapoint, lose_datapoint];
        for _ in 0..500 {
            let grad = compute_gradient(&weights, &dataset, EvalScale(1.0));
            println!(
                "current weights: {0}, current loss: {1}, gradient: {2}",
                weights,
                loss(&weights, &dataset, EvalScale(1.0)),
                grad,
            );
            let old_weights = weights.clone();
            weights -= grad;
            assert!(
                loss(&weights, &dataset, EvalScale(1.0))
                    <= loss(&weights, &dataset, EvalScale(1.0))
            );
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
            let eval_scale = EvalScale(10000.0);
            let dataset = vec![Datapoint {
                position: vec![Feature::new_i8(1)],
                outcome: Outcome::new(outcome),
            }];
            let mut adam = Adam::new(&dataset, eval_scale);
            let weights = adam.optimize(&dataset, eval_scale, 20);
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
