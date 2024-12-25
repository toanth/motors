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

use colored::Color::TrueColor;
use colored::Colorize;
use gears::colorgrad::{BasisGradient, Gradient, LinearGradient};
use gears::games::Color;
use gears::general::board::Board;
use gears::general::common::{sigmoid, Tokens};
use gears::general::move_list::MoveList;
use gears::general::moves::ExtendedFormat::Standard;
use gears::general::moves::Move;
use gears::output::{Message, OutputBox, OutputOpts};
use gears::score::{Score, SCORE_LOST, SCORE_WON};
use gears::search::MpvType::{MainOfMultiple, OnlyLine, SecondaryLine};
use gears::search::NodeType::*;
use gears::search::{MpvType, NodeType, SearchInfo, SearchResult};
use gears::{colorgrad, GameState};
use indicatif::{ProgressBar, ProgressStyle};
use std::fmt;
use std::io::stdout;

#[derive(Debug)]
/// All UGI communication is done through stdout, but there can be additional outputs,
/// such as a logger, or human-readable printing to stderr
pub struct UgiOutput<B: Board> {
    pub(super) additional_outputs: Vec<OutputBox<B>>,
    pub(super) pretty: bool,
    previous_exact_info: Option<SearchInfo<B>>,
    gradient: LinearGradient,
    alt_grad: BasisGradient,
    progress_bar: Option<ProgressBar>,
}

impl<B: Board> Default for UgiOutput<B> {
    fn default() -> Self {
        Self {
            additional_outputs: vec![],
            pretty: false,
            previous_exact_info: None,
            gradient: score_gradient(),
            alt_grad: colorgrad::GradientBuilder::new()
                .html_colors(&["orange", "gold", "seagreen"])
                // .html_colors(&["red", "white", "green"]) // looks too much like the flag of italy
                .domain(&[0.0, 1.0])
                .build::<BasisGradient>()
                .unwrap(),
            progress_bar: None,
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
        self.clear_progress_bar();
        if !self.pretty {
            self.write_ugi(&res.to_string());
            return;
        }
        let mut move_text = res.chosen_move.to_extended_text(&res.pos, Standard).bold();
        if let Some(score) = res.score {
            move_text = move_text.color(color_for_score(score, &self.gradient));
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
    }

    pub fn write_currmove(&mut self, pos: &B, mov: B::Move, move_nr: usize) {
        if !pos.is_move_legal(mov) {
            return;
        }
        let move_nr = move_nr + 1; // UGI wants 1-indexed output
        let num_moves = pos.legal_moves_slow().num_moves();
        if !self.pretty {
            self.write_ugi("info currmove {mov} currmovenumber {move_nr}");
            return;
        }
        // TODO: Faster implementation for number of legal moves in Board trait?
        let bar = match &self.progress_bar {
            None => {
                let template = "{prefix}\n{bar:68.cyan/blue} {pos:>4}/{len:4}";
                self.progress_bar = Some(
                    ProgressBar::new(num_moves as u64)
                        .with_style(ProgressStyle::with_template(template).unwrap()),
                );
                &self.progress_bar.as_ref().unwrap()
            }
            Some(bar) => bar,
        };
        let move_string = mov.to_extended_text(&pos, Standard).bold();
        let elapsed = bar.elapsed().as_millis();
        let msg = format!("Searching {move_string} ({move_nr}/{num_moves}), {elapsed} ms elapsed in this aspiration window");
        bar.set_prefix(msg);
        bar.set_position(move_nr as u64);
    }

    pub fn write_search_info(&mut self, info: SearchInfo<B>) {
        self.clear_progress_bar();
        let exact = info.bound == Some(Exact);
        if !self.pretty {
            self.write_ugi(&info.to_string());
            if exact {
                self.previous_exact_info = Some(info);
            }
            return;
        }

        if info.mpv_type() != SecondaryLine
            && self
                .previous_exact_info
                .as_ref()
                .is_some_and(|i| i.depth != info.depth - 1)
        {
            self.previous_exact_info = None;
        }

        let score = pretty_score(
            info.score,
            info.bound,
            self.previous_exact_info.as_ref().map(|i| i.score),
            &self.gradient,
            info.pv_num == 0,
            true,
        );

        let mut time = info.time.as_secs_f64();
        let nodes = info.nodes.get() as f64 / 1_000_000.0;
        let diff_string = if let Some(prev) = &self.previous_exact_info {
            write_with_suffix(
                info.nodes.get() - prev.nodes.get(),
                info.mpv_type() == SecondaryLine,
            )
        } else {
            " ".repeat(8)
        };
        let nps = nodes / time;
        let nps_color = self.alt_grad.at(nps as f32 / 4.0);
        let [r, g, b, _] = nps_color.to_rgba8();
        let nps = format!("{nps:5.2}").color(TrueColor { r, g, b }).dimmed();
        let time_badness = 1.0 - (time + 1.0).log2() / 10.0;
        let [r, g, b, _] = self.alt_grad.at(time_badness as f32).to_rgba8();
        let mut in_seconds = true;
        if time >= 1000.0 {
            time /= 60.0;
            in_seconds = false;
        }
        let time = format!("{time:5.1}").color(TrueColor { r, g, b });
        let nodes = format!("{nodes:6.1}");

        let pv = pretty_pv(
            &info.pv,
            info.pos,
            self.previous_exact_info.as_ref().map(|i| i.pv.as_ref()),
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

        let mut iter = format!("{:>3}", info.depth);
        if !exact {
            iter = iter.dimmed().to_string();
            if let Some(FailLow) = info.bound {
                iter = iter
                    .color(color_for_score(SCORE_LOST, &self.gradient))
                    .to_string();
            } else if let Some(FailHigh) = info.bound {
                iter = iter
                    .color(color_for_score(SCORE_WON, &self.gradient))
                    .to_string();
            }
        } else if info.mpv_type() != SecondaryLine {
            iter = iter.bold().to_string();
        }
        let complete = if info.bound.is_some() {
            "   ".to_string()
        } else {
            "(*)".dimmed().to_string()
        };
        let seldepth = info.seldepth;

        let [r, g, b, _] = self
            .alt_grad
            .at(0.5 - info.hashfull as f32 / 1000.0)
            .to_rgba8();
        let tt = format!("{:5.1}", info.hashfull as f64 / 10.0)
            .to_string()
            .color(TrueColor { r, g, b })
            .dimmed();
        let text = format!(
                " {iter}{complete} {seldepth:>3} {multipv} {score:>8}  {time}{s}  {nodes}{M}{diff_string}  {nps}{M}  {tt}{p}  {pv}",
                s = if in_seconds {"s"} else {"m"}.dimmed(),
                M = "M".dimmed(),
                p = "%".dimmed(),
            );
        self.write_ugi(&text);
        if info.bound.is_none() {
            self.write_ugi(&"[(*) Iteration did not complete]".dimmed().to_string());
        }
        if info.mpv_type() != SecondaryLine && info.bound == Some(Exact) {
            self.previous_exact_info = Some(info);
        }
    }

    fn clear_progress_bar(&mut self) {
        if let Some(bar) = &self.progress_bar {
            bar.finish_and_clear();
        }
        self.progress_bar = None;
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

    pub fn show(&mut self, m: &dyn GameState<B>, opts: OutputOpts) -> bool {
        for output in &mut self.additional_outputs {
            output.show(m, opts);
        }
        self.additional_outputs
            .iter()
            .any(|o| !o.is_logger() && o.prints_board())
    }

    pub fn format(&mut self, m: &dyn GameState<B>, opts: OutputOpts) -> String {
        use std::fmt::Write;
        let mut res = String::new();
        for output in &mut self.additional_outputs {
            write!(&mut res, "{}", output.as_string(m, opts)).unwrap();
        }
        res
    }
}

pub fn suffix_for(val: isize, start: Option<usize>) -> (isize, &'static str) {
    if start.is_some() && val.unsigned_abs() < start.unwrap() {
        return (val, "");
    }
    let mut div = 1;
    for suffix in ["", "K", "M", "B"] {
        // just doing val = (val + 500) / 1000 each iteration would accumulate errors and round 499_500 to 1_000_000
        let new_val = (val + val.signum() * (div / 2)) / div;
        if new_val.abs() >= 1000 {
            div *= 1000;
        } else {
            return (new_val, suffix);
        }
    }
    (val, "???")
}

fn write_with_suffix(val: u64, dimmed: bool) -> String {
    let (new_val, suffix) = suffix_for(val as isize, None);
    let res = format!(" {:>7}", format!("(+{new_val:>3}{suffix})"));
    if dimmed {
        res.dimmed().to_string()
    } else {
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

pub fn color_for_score(score: Score, gradient: &LinearGradient) -> colored::Color {
    let sigmoid_score = sigmoid(score, 100.0) as f32;
    let color = gradient.at(sigmoid_score);
    let [r, g, b, _] = color.to_rgba8();
    TrueColor { r, g, b }
}

pub fn pretty_score(
    score: Score,
    bound: Option<NodeType>,
    previous: Option<Score>,
    gradient: &LinearGradient,
    main_line: bool,
    min_width: bool,
) -> String {
    let mut res = format!("{:>5}", score.0);
    if let Some(mate) = score.moves_until_game_won() {
        res = format!("#{mate}");
        if min_width {
            // 2 spaces because we don't print `cp`
            res = format!("  {:>5}", format!("#{mate}"))
        }
    } else if !min_width {
        res = score.0.to_string();
    }
    let mut res = res.color(color_for_score(score, gradient));
    if !main_line {
        res = res.dimmed();
    }
    // some (but not all) terminals have trouble with colored bold symbols, and using `bold` would remove the color in some cases.
    // For some reason, only using the ansi colore codes (.green(), .red(), etc) creates these problems, but true colors work fine
    let bound_string = match bound.unwrap_or(Exact) {
        FailHigh => "≥".color(color_for_score(SCORE_WON, gradient)).bold(),
        Exact => (if min_width { " " } else { "" }).into(),
        FailLow => "≤".color(color_for_score(SCORE_LOST, gradient)).bold(),
    };
    let res = if score.is_won_or_lost() {
        format!("{bound_string}{}", res.bold())
    } else {
        format!("{bound_string}{res}{}", "cp".dimmed())
    };
    // res = format!("{bound}{res}");
    if let Some(previous) = previous {
        if !main_line || bound != Some(Exact) {
            return res.to_string() + "  ";
        }
        // `sigmoid - sigmoid` instead of `sigmoid(diff)` to weight changes close to 0 stronger
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
        format!(
            "{res} {}",
            c.to_string().bold().color(TrueColor { r, g, b })
        )
    } else if min_width {
        res + "  "
    } else {
        res
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
            write!(res, "{}", move_nr.bold()).unwrap();
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
            return format!("{res} [Invalid PV move '{}']", mov.to_string().red());
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
            new_move = new_move.bold().to_string();
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
