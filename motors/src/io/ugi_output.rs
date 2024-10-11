/*
 *  Motors, a collection of board game engines.
 *  Copyright (C) 2024 ToTheAnd
 *
 *  Motors is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Motors is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Motors. If not, see <https://www.gnu.org/licenses/>.
 */

// TODO: Keep this is a global object instead? Would make it easier to print warnings from anywhere, simplify search sender design

use colored::{Colorize, CustomColor};
use colorgrad::{BasisGradient, Gradient, LinearGradient};
use gears::games::Color;
use gears::general::board::Board;
use gears::general::common::{sigmoid, Tokens};
use gears::general::moves::ExtendedFormat::Standard;
use gears::general::moves::Move;
use gears::output::{Message, OutputBox};
use gears::score::Score;
use gears::search::SearchInfo;
use gears::GameState;
use std::fmt;
use std::io::stdout;

#[derive(Debug)]
/// All UGI communication is done through stdout, but there can be additional outputs,
/// such as a logger, or human-readable printing to stderr
pub struct UgiOutput<B: Board> {
    pub(super) additional_outputs: Vec<OutputBox<B>>,
    pub(super) pretty: bool,
    previous_info: Option<SearchInfo<B>>,
    gradient: LinearGradient,
    alt_grad: BasisGradient,
}

impl<B: Board> Default for UgiOutput<B> {
    fn default() -> Self {
        Self {
            additional_outputs: vec![],
            pretty: false,
            previous_info: None,
            gradient: colorgrad::GradientBuilder::new()
                .html_colors(&["red", "gold", "green"])
                .domain(&[0.0, 1.0])
                .build::<LinearGradient>()
                .unwrap(),
            alt_grad: colorgrad::GradientBuilder::new()
                .html_colors(&["orange", "gold", "seagreen"])
                // .html_colors(&["red", "white", "green"]) // looks too much like the flag of italy
                .domain(&[0.0, 1.0])
                .build::<BasisGradient>()
                .unwrap(),
        }
    }
}

impl<B: Board> UgiOutput<B> {
    pub fn new(pretty: bool) -> Self {
        Self {
            pretty,
            ..Default::default()
        }
    }
}

impl<B: Board> UgiOutput<B> {
    /// Part of the UGI specification, but not the UCI specification

    pub(super) fn write_response(&mut self, response: &str) {
        self.write_ugi(&format!("response {response}"));
    }

    pub fn write_ugi(&mut self, message: &str) {
        use std::io::Stdout;
        use std::io::Write;
        // UGI is always done through stdin and stdout, no matter what the UI is.
        // TODO: Keep stdout mutex? Might make printing slightly faster and prevents everyone else from
        // accessing stdout, which is probably a good thing because it prevents sending invalid UCI commands
        println!("{message}");
        // Currently, `println` always flushes, but this behaviour should not be relied upon.
        _ = Stdout::flush(&mut stdout());
        for output in &mut self.additional_outputs {
            output.write_ugi_output(message, None);
        }
    }

    pub fn write_search_info(&mut self, info: SearchInfo<B>) {
        if self
            .previous_info
            .as_ref()
            .is_some_and(|i| i.depth != info.depth - 1)
        {
            self.previous_info = None;
        }
        let text = if self.pretty {
            let score = pretty_score(
                info.score,
                self.previous_info.as_ref().map(|i| i.score),
                &self.gradient,
            );

            let mut time = info.time.as_secs_f64();
            let nodes = info.nodes.get() as f64 / 1_000_000.0;
            let nps = nodes / time;
            let nps_color = self.alt_grad.at(nps as f32 / 3.0);
            let [nps_r, nps_g, nps_b, _] = nps_color.to_rgba8();
            let nps_color = CustomColor::new(nps_r, nps_g, nps_b);
            let nps = format!("{nps:5.2}").custom_color(nps_color);
            let time_badness = 1.0 - (time + 1.0).log2() / 10.0;
            let [t_r, t_g, t_b, _] = self.alt_grad.at(time_badness as f32).to_rgba8();
            let mut in_seconds = true;
            if time >= 1000.0 {
                time /= 60.0;
                in_seconds = false;
            }
            let time = format!("{time:5.1}").custom_color(CustomColor::new(t_r, t_g, t_b));
            let nodes = format!("{nodes:6.1}").bold();

            let pv = pretty_pv(
                &info.pv,
                info.pos,
                self.previous_info.as_ref().map(|i| i.pv.as_ref()),
            );
            let multipv = if info.pv_num == 0 {
                "    ".to_string()
            } else {
                format!("{:>4}", format!("({})", info.pv_num + 1))
            };
            let iter = info.depth.to_string().bold();
            let seldepth = info.seldepth;

            let [tt_r, tt_g, tt_b, _] = self
                .alt_grad
                .at(0.75 - info.hashfull as f32 / 2000.0)
                .to_rgba8();
            let tt = format!("{:5.1}", info.hashfull as f64 / 10.0)
                .to_string()
                .custom_color(CustomColor::new(tt_r, tt_g, tt_b))
                .dimmed();
            self.previous_info = Some(info);
            format!(
                " {iter:>3} {seldepth:>3} {multipv} {score}  {time}{s}  {nodes}{M}  {nps}  {tt}{p}  {pv}",
                s = if in_seconds {"s"} else {"m"}.dimmed(),
                M = "M".dimmed(),
                p = "%".dimmed(),
            )
        } else {
            info.to_string()
        };
        self.write_ugi(&text);
    }

    pub(super) fn write_ugi_input(&mut self, msg: Tokens) {
        for output in &mut self.additional_outputs {
            output.write_ugi_input(msg.clone(), None);
        }
    }

    pub fn write_message(&mut self, typ: Message, msg: &str) {
        for output in &mut self.additional_outputs {
            output.display_message(typ, msg);
        }
    }

    pub fn show(&mut self, m: &dyn GameState<B>) -> bool {
        for output in &mut self.additional_outputs {
            output.show(m);
        }
        self.additional_outputs
            .iter()
            .any(|o| !o.is_logger() && o.prints_board())
    }

    pub fn format(&mut self, m: &dyn GameState<B>) -> String {
        use std::fmt::Write;
        let mut res = String::new();
        for output in &mut self.additional_outputs {
            write!(&mut res, "{}", output.as_string(m)).unwrap();
        }
        res
    }
}

fn pretty_score(score: Score, previous: Option<Score>, gradient: &LinearGradient) -> String {
    use fmt::Write;
    let mut res = format!("{:>5}", score.0);
    if let Some(mate) = score.moves_until_game_won() {
        res = format!("   {:>4}", format!("#{mate}"))
    };
    let sigmoid_score = sigmoid(score, 100.0) as f32;
    let color = gradient.at(sigmoid_score);
    let [r, g, b, _] = color.to_rgba8();
    let mut res = res.custom_color(CustomColor::new(r, g, b)).to_string();
    if score.is_game_over_score() {
        res = res.bold().to_string();
    } else {
        write!(&mut res, "{}", "cp".dimmed()).unwrap();
    }
    if let Some(previous) = previous {
        // sigmoid - sigmoid instead of sigmoid(diff) to weight changes close to 0 stronger
        let x = (0.5 + 5.0 * (sigmoid_score - sigmoid(previous, 100.0) as f32)).clamp(0.0, 1.0);
        let color = gradient.at(x);
        let [r, g, b, _] = color.to_rgba8();
        let diff = score - previous;
        let c = if diff >= Score(10) {
            '🡩'
        } else if diff <= Score(-10) {
            '🡫'
        } else if diff > Score(0) {
            '🡭'
        } else if diff < Score(0) {
            '🡮'
        } else {
            '🡪'
        };
        format!(
            "{res} {}",
            c.to_string().bold().custom_color(CustomColor::new(r, g, b))
        )
    } else {
        res.to_string() + "  "
    }
}

fn pretty_pv<B: Board>(pv: &[B::Move], mut pos: B, previous: Option<&[B::Move]>) -> String {
    use fmt::Write;
    let mut same_so_far = true;
    let mut res = String::new();
    let pv = pv.iter();
    for (idx, mov) in pv.enumerate() {
        if !pos.is_move_legal(*mov) {
            return format!("{res} [Invalid PV move '{}'", mov.to_string().red());
        }
        // 'Alternative' would be cooler, but unfortunately most fonts struggle with unicode chess pieces,
        // especially in combination with bold / dimmed etc
        let mut new_move = mov.to_extended_text(&pos, Standard);
        let previous = previous
            .and_then(|p| p.get(idx))
            .copied()
            .unwrap_or_default();
        if previous == *mov {
            new_move = new_move.dimmed().to_string();
        } else if same_so_far {
            new_move = new_move.bold().to_string();
            same_so_far = false;
        }
        if pos.active_player().is_first() {
            let move_nr = format!(" {}.", pos.fullmove_ctr() + 1);
            write!(&mut res, "{}", move_nr.dimmed()).unwrap();
        } else if idx == 0 {
            let move_nr = format!(" {}. ...", pos.fullmove_ctr() + 1);
            write!(&mut res, "{}", move_nr.dimmed()).unwrap();
        } else {
            res.push(' ');
        }
        write!(&mut res, "{new_move}").unwrap();
        pos = pos.make_move(*mov).unwrap();
    }
    res
}
