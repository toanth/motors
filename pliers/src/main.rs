use derive_more::{Add, AddAssign, Deref, DerefMut, Display, Sub, SubAssign};
use std::fmt::Formatter;
use std::ops::{Mul, MulAssign};

type Float = f64;

/// The result of calling the eval function.
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
struct CpScore(Float);

impl Display for CpScore {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.1}cp", self.0)
    }
}

/// The wr prediction, based on the CpScore (between 0 and 1).
#[derive(Debug, Copy, Clone, PartialOrd, PartialEq)]
struct WrScore(Float);

/// `WrScore` is used for the converted score returned by the eval, `Outcome` for the actual outcome
type Outcome = WrScore;

impl WrScore {
    fn new(val: Float) -> Self {
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
struct EvalScale(Float);

fn sigmoid(x: Float, scale: EvalScale) -> Float {
    1.0 / (1.0 + (-x / scale.0).exp())
}

fn cp_to_wr(cp: CpScore, eval_scale: EvalScale) -> WrScore {
    WrScore(sigmoid(cp.0 as Float, eval_scale))
}

fn sample_delta(wr_prediction: WrScore, outcome: Outcome) -> Float {
    wr_prediction.0 - outcome.0
}

fn sample_loss(wr_prediction: WrScore, outcome: Outcome) -> Float {
    let delta = sample_delta(wr_prediction, outcome);
    delta * delta
}

fn sample_loss_for_cp(eval: CpScore, outcome: Outcome, eval_scale: EvalScale) -> Float {
    let wr_prediction = cp_to_wr(eval, eval_scale);
    sample_loss(wr_prediction, outcome)
}

#[derive(
    Debug, Display, Default, Copy, Clone, PartialOrd, PartialEq, Add, AddAssign, Sub, SubAssign,
)]
struct Weight(Float);

#[derive(Debug, Clone, Deref, DerefMut)]
struct Weights(Vec<Weight>);

type Gradient = Weights;

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

impl Weights {
    fn update(&mut self, other: &Position, factor: Float) {
        for (w, rhs) in self.iter_mut().zip(other.iter()) {
            w.0 += rhs.0 as Float * factor;
        }
    }
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
struct Feature(i32);

type Position = Vec<Feature>;

struct Datapoint {
    position: Position,
    outcome: Outcome,
}

type Dataset = Vec<Datapoint>;

fn cp_eval_for_weights(weights: &Weights, position: &Position) -> CpScore {
    debug_assert_eq!(weights.len(), position.len());
    CpScore(
        weights
            .iter()
            .zip(position.iter())
            .map(|(w, feature)| w.0 * feature.0 as Float)
            .sum::<Float>(),
    )
}

fn wr_prediction_for_weights(
    weights: &Weights,
    position: &Position,
    eval_scale: EvalScale,
) -> WrScore {
    let eval = cp_eval_for_weights(weights, position);
    cp_to_wr(eval, eval_scale)
}

fn loss(weights: &Weights, dataset: &Dataset, scaling_factor: EvalScale) -> Float {
    let mut res = Float::default();
    for datapoint in dataset.iter() {
        let eval = wr_prediction_for_weights(weights, &datapoint.position, scaling_factor);
        let loss = sample_loss(eval, datapoint.outcome);
        res += loss;
        // println!(
        //     "- eval: {eval}, current loss: {loss}, total: {res}, result: {}",
        //     res / dataset.len() as Float
        // );
    }
    res / dataset.len() as Float
}

/// Computes the gradient of the loss function:
/// The loss function of a single sample is `(sigmoid(sample, scale) - outcome) ^ 2`,
/// so per the chain rule, the derivative is `2 * (sigmoid(sample, scale) - outcome) * sigmoid'(sample, scale)`,
/// where the derivative of the sigmoid, sigmoid', is `scale * sigmoid(sample, scale) * (1 - sigmoid(sample, scale)`.
fn compute_gradient(weights: &Weights, dataset: &Dataset, scaling_factor: EvalScale) -> Gradient {
    let mut grad = Weights(vec![Weight::default(); weights.len()]);
    for data in dataset.iter() {
        let wr_prediction = wr_prediction_for_weights(weights, &data.position, scaling_factor).0;

        // the 2 is a constant factor and could be dropped because we don't need to preserve the magnitude of the gradient,
        // but let's be correct and keep it.
        let scaled_delta = 2.0
            * scaling_factor.0
            * (wr_prediction - data.outcome.0)
            * wr_prediction
            * (1.0 - wr_prediction);
        grad.update(&data.position, scaled_delta);
    }
    grad
}

fn main() {
    let mut weights = Weights(vec![Weight(0.4), Weight(1.0), Weight(2.0)]);
    let draw_datapoint = Datapoint {
        position: vec![Feature(0), Feature(0), Feature(0)],
        outcome: Outcome::new(0.5),
    };
    let lose_datapoint = Datapoint {
        position: vec![Feature(-1), Feature(-1), Feature(0)],
        outcome: Outcome::new(0.0),
    };
    let win_datapoint = Datapoint {
        position: vec![Feature(1), Feature(1), Feature(0)],
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
        // TODO: Step size control (starting with adam, then adamw?)
        weights -= grad;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering::Equal;

    #[test]
    // testcase that contains only 1 position with only 1 feature
    pub fn trivial_test() {
        let scaling_factor = EvalScale(100.0);
        for feature in [1, 2, -1, 0] {
            for initial_weight in [0.0, 0.1, 100.0, -1.2] {
                for outcome in [0.0, 0.5, 1.0, 0.9, 0.499] {
                    let mut weights = Weights(vec![Weight(initial_weight)]);
                    let position = vec![Feature(feature)];
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
            let position = vec![Feature(1), Feature(-1), Feature(2)];
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
        for outcome in [0.0, 0.5, 1.0] {
            let mut weights = Weights(vec![Weight(123.987), Weight(-987.123)]);
            let position = vec![Feature(1), Feature(-1)];
            let datapoint = Datapoint {
                position: position.clone(),
                outcome: Outcome::new(outcome),
            };
            let dataset = vec![datapoint];
            for _ in 0..100 {
                let grad = compute_gradient(&weights, &dataset, EvalScale(1000.0));
                let old_weights = weights.clone();
                weights -= grad.clone();
                assert!(
                    loss(&weights, &dataset, EvalScale(1.0))
                        <= loss(&old_weights, &dataset, EvalScale(1.0))
                );
            }
            let loss = loss(&weights, &dataset, EvalScale(1.0));
            assert!(loss <= 0.01);
            if outcome == 0.5 {
                assert_eq!(weights[0].0.signum(), weights[1].0.signum());
                assert!((weights[0].0.abs() - weights[1].0.abs()).abs() <= 0.00000001);
            } else {
                assert_eq!(weights[0].0.signum(), -weights[1].0.signum());
            }
        }
    }
}
