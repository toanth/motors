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

use gears::colorgrad::{BasisGradient, Gradient, LinearGradient};
use gears::crossterm::style::Color::Rgb;
use gears::crossterm::style::Stylize;
use gears::games::Color;
use gears::general::board::Board;
use gears::general::common::{sigmoid, ColorMsg, Tokens};
use gears::general::moves::ExtendedFormat::Standard;
use gears::general::moves::Move;
use gears::output::{Message, OutputBox};
use gears::score::Score;
use gears::search::MpvType::{MainOfMultiple, OnlyLine, SecondaryLine};
use gears::search::{MpvType, SearchInfo, SearchResult};
use gears::{colorgrad, GameState};
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
            gradient: score_gradient(),
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

    pub fn write_search_res(&mut self, res: SearchResult<B>) {
        if self.pretty {
            let mut move_text = res
                .chosen_move
                .to_extended_text(&res.pos, Standard)
                .important();
            if let Some(score) = res.score {
                move_text = move_text.with(color_for_score(score, &self.gradient));
            }
            let mut msg = format!("Chosen move: {move_text}",);
            if let Some(ponder) = res.ponder_move() {
                let new_pos = res
                    .pos
                    .make_move(res.chosen_move)
                    .expect("Search returned illegal move");
                msg += &format!(
                    " (expected response: {})",
                    ponder.to_extended_text(&new_pos, Standard)
                )
                .dimmed()
                .to_string();
            }
            self.write_ugi(&msg)
        } else {
            self.write_ugi(&res.to_string());
        }
    }

    pub fn write_search_info(&mut self, info: SearchInfo<B>) {
        if info.mpv_type() != SecondaryLine
            && self
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
                info.pv_num == 0,
                true,
            );

            let mut time = info.time.as_secs_f64();
            let nodes = info.nodes.get() as f64 / 1_000_000.0;
            let nps = nodes / time;
            let nps_color = self.alt_grad.at(nps as f32 / 4.0);
            let [r, g, b, _] = nps_color.to_rgba8();
            let nps = format!("{nps:5.2}").with(Rgb { r, g, b }).dimmed();
            let time_badness = 1.0 - (time + 1.0).log2() / 10.0;
            let [r, g, b, _] = self.alt_grad.at(time_badness as f32).to_rgba8();
            let mut in_seconds = true;
            if time >= 1000.0 {
                time /= 60.0;
                in_seconds = false;
            }
            let time = format!("{time:5.1}").with(Rgb { r, g, b });
            let nodes = format!("{nodes:6.1}");

            let pv = pretty_pv(
                &info.pv,
                info.pos,
                self.previous_info.as_ref().map(|i| i.pv.as_ref()),
                info.mpv_type(),
            );
            let mut multipv = if info.mpv_type() == OnlyLine {
                "    ".to_string()
            } else {
                format!("{:>4}", format!("({})", info.pv_num + 1))
            };
            if info.mpv_type() == SecondaryLine {
                multipv = multipv.dimmed().to_string();
            }
            // bold after formatting because crossterms seems to count the control characters towards the format width
            let iter = format!("{:>3}", info.depth).important();
            let seldepth = info.seldepth;

            let [r, g, b, _] = self
                .alt_grad
                .at(0.75 - info.hashfull as f32 / 2000.0)
                .to_rgba8();
            let tt = format!("{:5.1}", info.hashfull as f64 / 10.0)
                .to_string()
                .with(Rgb { r, g, b })
                .dimmed();
            if info.mpv_type() != SecondaryLine {
                self.previous_info = Some(info);
            }
            format!(
                " {iter} {seldepth:>3} {multipv} {score}  {time}{s}  {nodes}{M}  {nps}  {tt}{p}  {pv}",
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

pub fn score_gradient() -> LinearGradient {
    colorgrad::GradientBuilder::new()
        .html_colors(&["red", "gold", "green"])
        .domain(&[0.0, 1.0])
        .build::<LinearGradient>()
        .unwrap()
}

pub fn color_for_score(score: Score, gradient: &LinearGradient) -> gears::crossterm::style::Color {
    let sigmoid_score = sigmoid(score, 100.0) as f32;
    let color = gradient.at(sigmoid_score);
    let [r, g, b, _] = color.to_rgba8();
    Rgb { r, g, b }
}

pub fn pretty_score(
    score: Score,
    previous: Option<Score>,
    gradient: &LinearGradient,
    main_line: bool,
    min_width: bool,
) -> String {
    use fmt::Write;
    let mut res = format!("{:>5}", score.0);
    if let Some(mate) = score.moves_until_game_won() {
        res = format!("#{mate}");
        if min_width {
            res = format!("   {:>4}", format!("#{mate}"))
        }
    } else if !min_width {
        res = score.0.to_string();
    }
    let mut res = res.with(color_for_score(score, gradient)).to_string();
    if !main_line {
        res = res.dimmed().to_string();
    }
    if score.is_game_over_score() {
        res = res.important().to_string();
    } else {
        write!(&mut res, "{}", "cp".dimmed()).unwrap();
    }
    if let Some(previous) = previous {
        if !main_line {
            return res.to_string() + "  ";
        }
        // sigmoid - sigmoid instead of sigmoid(diff) to weight changes close to 0 stronger
        let x = 0.5 + 2.0 * (sigmoid(score, 100.0) as f32 - sigmoid(previous, 100.0) as f32);
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
        format!("{res} {}", c.to_string().important().with(Rgb { r, g, b }))
    } else if min_width {
        res.to_string() + "  "
    } else {
        res.to_string()
    }
}

fn write_move_nr(
    res: &mut String,
    move_nr: usize,
    first_move: bool,
    first_player: bool,
    mpv_type: MpvType,
) {
    use fmt::Write;
    let mut write = |move_nr: String| {
        if mpv_type == MainOfMultiple {
            write!(res, "{}", move_nr.important()).unwrap();
        } else {
            write!(res, "{}", move_nr.dimmed()).unwrap();
        }
    };
    if first_player {
        write(format!(" {}.", move_nr));
    } else if first_move {
        write(format!(" {}. ...", move_nr));
    } else {
        res.push(' ');
    }
}

fn pretty_pv<B: Board>(
    pv: &[B::Move],
    mut pos: B,
    previous: Option<&[B::Move]>,
    mpv_type: MpvType,
) -> String {
    use fmt::Write;
    let mut same_so_far = true;
    let mut res = String::new();
    let pv = pv.iter();
    for (idx, mov) in pv.enumerate() {
        if !pos.is_move_legal(*mov) {
            return format!("{res} [Invalid PV move '{}'", mov.to_string().error());
        }
        // 'Alternative' would be cooler, but unfortunately most fonts struggle with unicode chess pieces,
        // especially in combination with bold / dim etc
        let mut new_move = mov.to_extended_text(&pos, Standard);
        let previous = previous
            .and_then(|p| p.get(idx))
            .copied()
            .unwrap_or_default();
        if previous == *mov || mpv_type == SecondaryLine {
            new_move = new_move.dimmed().to_string();
        } else if same_so_far && mpv_type != SecondaryLine {
            new_move = new_move.important().to_string();
            same_so_far = false;
        }
        write_move_nr(
            &mut res,
            pos.fullmove_ctr_1_based(),
            idx == 0,
            pos.active_player().is_first(),
            mpv_type,
        );
        write!(&mut res, "{new_move}").unwrap();
        pos = pos.make_move(*mov).unwrap();
    }
    res
}
