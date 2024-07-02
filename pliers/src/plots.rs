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

//! Everything related to generating plots.

/// Losses over time
pub type Losses = Vec<Float>;

/// Collects statistics about the type tune.
#[derive(Debug, Default)]
pub struct Statistics {
    /// losses as calculated by the default loss function
    pub losses: Losses,
    /// maximum weight changes
    pub max_deltas: Vec<Float>,
    /// quadratic losses. The ideal value is 0.
    pub quadratic_losses: Vec<Float>,
}

use crate::gd::Float;
use plotters::backend::SVGBackend;
use plotters::prelude::*;
use std::path::PathBuf;

/// Generate a plot for the loss over time.
pub fn plot_losses(
    loss: &[Float],
    stepsize: usize,
    filename: &str,
    title: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let (min, max) = (
        loss.iter()
            .min_by(|a, b| a.partial_cmp(&b).unwrap())
            .unwrap(),
        loss.iter()
            .max_by(|a, b| a.partial_cmp(&b).unwrap())
            .unwrap(),
    );
    let max_epoch = loss.len() * stepsize;
    let root = SVGBackend::new(filename, (640, 640)).into_drawing_area();
    root.fill(&WHITE).unwrap();
    let root = root.margin(10, 10, 10, 10);
    let mut chart = ChartBuilder::on(&root)
        .caption(title, ("sans-serif", 40).into_font())
        .x_label_area_size(50)
        .y_label_area_size(100)
        .build_cartesian_2d(0..max_epoch, (*min..*max).log_scale())?;
    chart.configure_mesh().y_labels(10).draw()?;
    chart.draw_series(LineSeries::new(
        (0..)
            .zip(loss.iter())
            .map(|(idx, loss)| (idx * stepsize, *loss)),
        &BLUE,
    ))?;
    root.present()?;
    Ok(())
}

/// Generate plots for the collected statistics
pub fn plot_statistics(
    statistics: &Statistics,
    stepsize: usize,
    path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    plot_losses(
        &statistics.losses[1..],
        stepsize,
        path.join("loss.svg").to_str().ok_or("")?,
        "Training Loss",
    )?;
    plot_losses(
        &statistics.quadratic_losses[1..],
        stepsize,
        path.join("quadratic_loss.svg").to_str().ok_or("")?,
        "Quadratic Training Loss",
    )?;

    plot_losses(
        &statistics.max_deltas[1..],
        stepsize,
        path.join("max_delta.svg").to_str().ok_or("")?,
        "Maximum Weight Change",
    )?;
    Ok(())
}
