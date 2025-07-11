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

use gears::colored::Color::TrueColor;
use gears::colored::Colorize;
use gears::colorgrad::{BasisGradient, Gradient, LinearGradient};
use gears::games::CharType::Unicode;
use gears::games::Color;
use gears::general::board::{Board, BoardHelpers};
use gears::general::common::{Tokens, sigmoid};
use gears::general::moves::ExtendedFormat::Standard;
use gears::general::moves::Move;
use gears::itertools::Itertools;
use gears::output::{Message, OutputBox, OutputOpts};
use gears::score::{SCORE_LOST, SCORE_WON, Score};
use gears::search::MpvType::{MainOfMultiple, OnlyLine, SecondaryLine};
use gears::search::NodeType::*;
use gears::search::{Budget, DepthPly, MpvType, NodeType, NodesLimit, SearchInfo, SearchResult};
use gears::{GameState, colored, colorgrad};
use indicatif::{ProgressBar, ProgressStyle};
use std::fmt::Write;
use std::io::stdout;
use std::time::Duration;
use std::{fmt, mem};

#[derive(Debug)]
struct TypeErasedSearchInfo {
    budget: Budget,
    iterations: DepthPly,
    seldepth: DepthPly,
    time: Duration,
    nodes: NodesLimit,
    pv_num: usize,
    score: Score,
    hashfull: usize,
    num_threads: f64,
    bound: Option<NodeType>,
}

impl TypeErasedSearchInfo {
    fn new<B: Board>(info: SearchInfo<B>) -> Self {
        Self {
            budget: info.budget,
            iterations: info.iterations,
            seldepth: info.seldepth,
            time: info.time,
            nodes: info.nodes,
            pv_num: info.pv_num,
            score: info.score,
            hashfull: info.hashfull,
            num_threads: info.num_threads as f64,
            bound: info.bound,
        }
    }

    fn effective_branching_factor(&self) -> f64 {
        // this method of computing the effective branching factor is somewhat flawed, but it's what most engines do,
        // so for the sake of comparability we do this as well
        let iters = self.iterations.get() as u64;
        if iters == 0 {
            return 0.0; // I hate NaNs.
        }
        // subtract the depth to not count the root node, which means the branching factor for depth 1 is the number of legal moves
        ((self.nodes.get() - iters) as f64 / self.num_threads).powf(1.0 / iters as f64)
    }
}

#[derive(Debug)]
struct TypeErasedUgiOutput {
    pretty: bool,
    gradient: LinearGradient,
    alt_grad: BasisGradient,
    progress_bar: Option<ProgressBar>,
    previous_exact_info: Option<TypeErasedSearchInfo>,
    previous_exact_pv_end_pos: Option<(String, String)>,
}

impl Default for TypeErasedUgiOutput {
    fn default() -> Self {
        Self {
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
            previous_exact_pv_end_pos: None,
        }
    }
}

impl TypeErasedUgiOutput {
    #[allow(clippy::too_many_arguments)]
    fn show_bar(
        &mut self,
        num_moves: usize,
        top_moves: Option<&str>,
        pretty_variation: &str,
        eval: Score,
        alpha: Score,
        beta: Score,
        curr_pos: Option<&str>,
        root_pos: Option<&str>,
    ) -> &ProgressBar {
        use fmt::Write;
        let bar = self.progress_bar.get_or_insert_with(|| {
            let template = "{prefix}\n{bar:68.cyan/blue} {pos:>3}/{len:3}";
            ProgressBar::new(num_moves as u64).with_style(ProgressStyle::with_template(template).unwrap())
        });
        let elapsed = bar.elapsed().as_millis();
        let eval = pretty_score(eval, None, None, &self.gradient, true, false);
        let alpha = pretty_score(alpha, None, None, &self.gradient, true, false);
        let beta = pretty_score(beta, None, None, &self.gradient, true, false);
        let score_string = format!("{eval} with bounds ({alpha}, {beta})");
        let mut message = String::new();
        write!(
            message,
            "{0}{elapsed:>5}{1} {pretty_variation} [{score_string}]",
            "[".dimmed(),
            "ms in this AW] Searching".dimmed(),
        )
        .unwrap();
        if let Some(str) = top_moves {
            write!(message, "{str}").unwrap();
        }
        Self::write_boards(&mut message, curr_pos, root_pos, &self.previous_exact_pv_end_pos);
        bar.set_prefix(message);
        bar
    }

    fn write_boards(msg: &mut String, curr_pos: Option<&str>, root_pos: Option<&str>, prev: &Option<(String, String)>) {
        let Some((pv_pos, fen)) = prev else {
            return;
        };
        let spacer = " ".repeat(15);
        writeln!(msg, "\nPosition at the end of the PV: '{}'", fen.dimmed()).unwrap();
        if let Some(curr_pos) = curr_pos {
            let root_pos = root_pos.unwrap();
            let mut boards = String::new();
            for (first, (second, third)) in root_pos.lines().zip(pv_pos.lines().zip(curr_pos.lines())) {
                writeln!(boards, "{first}{spacer}{second}{spacer}{third}").unwrap();
            }
            writeln!(msg, "{boards}").unwrap();
        }
    }

    // this is a pretty large function, and instantiating it for each game would make it the 5th largest function in terms of generated llvm lines.
    fn format_pretty_search_info_non_generic(
        &mut self,
        info: &TypeErasedSearchInfo,
        pv: &str,
        mpv_type: MpvType,
    ) -> String {
        assert!(self.pretty);
        let exact = info.bound == Some(Exact);

        let important = mpv_type != SecondaryLine && exact;

        if mpv_type != SecondaryLine
            && self.previous_exact_info.as_ref().is_some_and(|i| i.iterations != info.iterations - 1)
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
        let nodes = info.nodes.get();
        let diff_string = if let Some(prev) = &self.previous_exact_info {
            write_with_suffix(info.nodes.get().saturating_sub(prev.nodes.get()), !important)
        } else {
            " ".repeat(8)
        };
        let nps = nodes as f64 / 1_000_000.0 / time;
        let nps_color = self.alt_grad.at((nps / (4.0 * info.num_threads)) as f32);
        let [r, g, b, _] = nps_color.to_rgba8();
        let nps = format!("{nps:5.2}").color(TrueColor { r, g, b }).dimmed();
        let time_badness = 1.0 - (time + 1.0).log2() / 10.0;
        let [r, g, b, _] = self.alt_grad.at(time_badness as f32).to_rgba8();
        let mut in_seconds = true;
        if time >= 1000.0 {
            time /= 60.0;
            in_seconds = false;
        }
        let time = format!("{time:6.2}").color(TrueColor { r, g, b });
        let nodes = format!("{nodes:12}");

        let mut multipv =
            if mpv_type == OnlyLine { "    ".to_string() } else { format!("{:>4}", format!("({})", info.pv_num + 1)) };
        if mpv_type == SecondaryLine {
            multipv = multipv.dimmed().to_string();
        }

        let mut iter = format!("{:>3}", info.iterations);
        if !exact {
            iter = iter.dimmed().to_string();
            // use color_for_score instead of `.green()` etc because some terminals struggle with non-true colors and dimmed/bold text.
            if let Some(FailLow) = info.bound {
                iter = iter.color(color_for_score(SCORE_LOST, &self.gradient)).to_string();
            } else if let Some(FailHigh) = info.bound {
                iter = iter.color(color_for_score(SCORE_WON, &self.gradient)).to_string();
            }
        } else if mpv_type != SecondaryLine {
            iter = iter.bold().to_string();
        }
        let complete = if info.bound.is_some() { "   ".to_string() } else { "(*)".dimmed().to_string() };
        let budget = info.budget;
        let seldepth = info.seldepth;

        let [r, g, b, _] = self.alt_grad.at(0.5 - info.hashfull as f32 / 1000.0).to_rgba8();
        let tt = format!("{:5.1}", info.hashfull as f64 / 10.0).to_string().color(TrueColor { r, g, b }).dimmed();
        let branching = format!("{:>6.2}", info.effective_branching_factor()).dimmed();
        format!(
            " {iter}{complete} {budget:>5}/{seldepth:<3} {multipv} {score:>8}  {time}{s}{nodes}{diff_string}  {nps}{M}  {branching} {tt}{p}  {pv}",
            s = if in_seconds { "s" } else { "m" }.dimmed(),
            M = "M".dimmed(),
            p = "%".dimmed(),
        )
    }

    fn clear_progress_bar(&mut self) {
        if let Some(bar) = &self.progress_bar {
            bar.finish_and_clear();
        }
        self.progress_bar = None;
    }
}

pub trait AbstractUgiOutput {
    fn write_ugi(&mut self, msg: &fmt::Arguments);

    fn write_ugi_input(&mut self, msg: Tokens);
}

impl<B: Board> AbstractUgiOutput for UgiOutput<B> {
    fn write_ugi(&mut self, message: &fmt::Arguments) {
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

    fn write_ugi_input(&mut self, msg: Tokens) {
        for output in &mut self.additional_outputs {
            output.write_ugi_input(msg.clone(), None);
        }
    }
}

#[derive(Debug)]
/// All UGI communication is done through stdout, but there can be additional outputs,
/// such as a logger, or human-readable printing to stderr
pub struct UgiOutput<B: Board> {
    type_erased: TypeErasedUgiOutput,
    pub(super) additional_outputs: Vec<OutputBox<B>>,
    previous_exact_pv: Option<Vec<B::Move>>,
    top_moves: Vec<(B::Move, Score)>,
    pub show_refutation: bool,
    pub show_currline: bool,
    pub currline_null_moves: bool,
    pub show_debug_output: bool,
    pub minimal: bool,
}

impl<B: Board> Default for UgiOutput<B> {
    fn default() -> Self {
        Self {
            additional_outputs: vec![],
            previous_exact_pv: None,
            type_erased: TypeErasedUgiOutput::default(),
            show_refutation: false,
            show_currline: false,
            currline_null_moves: true,
            top_moves: vec![],
            show_debug_output: false,
            minimal: false,
        }
    }
}

impl<B: Board> UgiOutput<B> {
    pub fn new(pretty: bool, debug: bool) -> Self {
        let mut res = Self::default();
        res.type_erased.pretty = pretty;
        res.show_debug_output = debug;
        res
    }

    pub fn new_search(&mut self) {
        self.top_moves.clear();
        self.previous_exact_pv = None;
    }

    pub fn set_pretty(&mut self, pretty: bool) {
        self.type_erased.pretty = pretty;
    }

    pub fn set_debug(&mut self, debug: bool) {
        self.show_debug_output = debug;
    }

    pub fn write_search_res(&mut self, res: &SearchResult<B>) {
        self.type_erased.clear_progress_bar();
        if !self.type_erased.pretty {
            self.write_ugi(&format_args!("{res}"));
            return;
        }
        let mut move_text = res.chosen_move.to_extended_text(&res.pos, Standard).bold();
        move_text = move_text.color(color_for_score(res.score, &self.type_erased.gradient));
        let mut msg = format!("Chosen move: {move_text}",);
        if let Some(ponder) = res.ponder_move() {
            let new_pos = res.pos.clone().make_move(res.chosen_move).expect("Search returned illegal move");
            msg +=
                &format!(" (expected response: {})", ponder.to_extended_text(&new_pos, Standard)).dimmed().to_string();
        }
        self.write_ugi(&format_args!("{msg}"))
    }

    fn can_show_currline(&mut self) -> bool {
        (self.show_currline || self.type_erased.pretty) & !self.minimal
    }

    fn can_show_refutation(&mut self) -> bool {
        (self.show_refutation || self.type_erased.pretty) & !self.minimal
    }

    pub fn write_currmove(&mut self, pos: &B, mov: B::Move, move_nr: usize, score: Score, alpha: Score, beta: Score) {
        if !self.can_show_currline() || !pos.is_move_legal(mov) {
            return;
        }
        // UGI wants 1-indexed output, but we've already counted the move, so move_nr is 1-indexed
        let num_moves = pos.num_legal_moves();
        if !self.type_erased.pretty {
            self.write_ugi(&format_args!("info currmove {0} currmovenumber {move_nr}", mov.compact_formatter(pos)));
            return;
        }
        let (variation, _) = pretty_variation(&[mov], pos.clone(), None, None, Exact);
        let bar = self.type_erased.show_bar(num_moves, None, &variation, score, alpha, beta, None, None);
        bar.set_position(move_nr as u64);
    }

    pub fn write_currline(
        &mut self,
        pos: &B,
        variation: impl Iterator<Item = B::Move>,
        eval: Score,
        alpha: Score,
        beta: Score,
    ) {
        use std::fmt::Write;
        if !self.can_show_currline() {
            return;
        }
        if !self.type_erased.pretty {
            let line = format_variation_noninteractive(pos.clone(), variation, self.currline_null_moves);
            // We only send search results from the main thread, no matter how many threads are searching.
            // And we're also not inspecting other threads' PVs from the main thread.
            self.write_ugi(&format_args!("info cpu 1 currline{line}"));
            return;
        }
        let num_legal = pos.num_legal_moves();
        let variation = variation.collect_vec();
        let (variation, end_pos) = pretty_variation(&variation, pos.clone(), None, None, Exact);
        let end_pos = end_pos.as_diagram(Unicode, false);
        let root_pos = pos.as_diagram(Unicode, false);
        let mut top_moves = "\nTop moves: ".to_string();
        for (i, (m, score)) in self.top_moves.iter().enumerate() {
            let score = pretty_score(*score, None, None, &self.type_erased.gradient, false, false);
            if i > 0 {
                write!(top_moves, ", ").unwrap();
            }
            write!(top_moves, "{0} [{score}]", m.extended_formatter(pos, Standard)).unwrap();
        }
        let top_moves = if self.top_moves.is_empty() { None } else { Some(top_moves.as_ref()) };
        _ = self.type_erased.show_bar(
            num_legal,
            top_moves,
            &variation,
            eval,
            alpha,
            beta,
            Some(&end_pos),
            Some(&root_pos),
        );
    }

    pub fn write_refutation(&mut self, pos: &B, refuted_move: B::Move, score: Score, move_num: usize) {
        if move_num == 0 {
            self.top_moves.clear();
        }
        if !self.can_show_refutation() {
            return;
        }
        self.top_moves.push((refuted_move, score));
        if !self.type_erased.pretty {
            self.write_ugi(&format_args!("info refutation {}", refuted_move.compact_formatter(pos)));
        }
    }

    pub fn write_search_info(&mut self, mut info: SearchInfo<B>) {
        if self.minimal && !info.final_info {
            return;
        }
        if !self.type_erased.pretty {
            self.write_ugi(&format_args!("{info}"));
            return;
        }
        self.type_erased.clear_progress_bar();
        let mpv_type = info.mpv_type();
        let pv = mem::take(&mut info.pv);
        let (pv_string, end_pos) = pretty_variation(
            pv,
            info.pos.clone(),
            self.previous_exact_pv.as_ref().map(|i| i.as_ref()),
            Some(mpv_type),
            info.bound.unwrap_or(Exact),
        );
        let info = TypeErasedSearchInfo::new(info);
        let text = self.type_erased.format_pretty_search_info_non_generic(&info, &pv_string, mpv_type);
        self.write_ugi(&format_args!("{text}"));
        if info.bound.is_none() {
            self.write_ugi(&format_args!("{}", "[(*) Iteration did not complete]".dimmed()));
        }

        if mpv_type != SecondaryLine && info.bound == Some(Exact) {
            self.type_erased.previous_exact_info = Some(info);
            self.previous_exact_pv = Some(pv.into());
            self.type_erased.previous_exact_pv_end_pos = Some((end_pos.as_diagram(Unicode, false), end_pos.as_fen()));
        }
    }

    pub fn write_message(&mut self, typ: Message, msg: &fmt::Arguments) {
        for output in &mut self.additional_outputs {
            output.display_message(typ, msg);
        }
    }

    pub fn show(&mut self, m: &dyn GameState<B>, opts: OutputOpts) {
        for output in &mut self.additional_outputs {
            output.show(m, opts);
        }
    }
}

fn format_variation_noninteractive<B: Board>(
    mut pos: B,
    variation: impl Iterator<Item = B::Move>,
    allow_nullmoves: bool,
) -> String {
    let mut line = String::new();
    for mov in variation {
        let old_pos = pos.clone();
        if mov.is_null() {
            if allow_nullmoves {
                pos = pos.make_nullmove().unwrap();
            } else {
                break;
            }
        } else {
            pos = pos.make_move(mov).unwrap();
        }
        write!(line, " {}", mov.compact_formatter(&old_pos)).unwrap();
    }
    line
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
    if dimmed { res.dimmed().to_string() } else { res }
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
    use std::fmt::Write;
    let res = if let Some(mate) = score.moves_until_game_won() {
        if min_width {
            // 2 spaces because we don't print `cp`
            format!("  {:>5}", format!("#{mate}"))
        } else {
            format!("#{mate}")
        }
    } else if min_width {
        format!("{:>5}", score.0)
    } else {
        score.0.to_string()
    };
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
    let mut res = if score.is_won_or_lost() {
        format!("{bound_string}{}", res.bold())
    } else {
        format!("{bound_string}{res}{}", "cp".dimmed())
    };
    if let Some(previous) = previous {
        if !main_line || bound != Some(Exact) {
            return res + "   ";
        }
        // use both `sigmoid - sigmoid` and `sigmoid(diff)` to weight changes close to 0 stronger
        let x = ((0.5 + 2.0 * (sigmoid(score, 100.0) as f32 - sigmoid(previous, 100.0) as f32))
            + sigmoid(score - previous, 50.0) as f32)
            / 2.0;
        let color = gradient.at(x);
        let [r, g, b, _] = color.to_rgba8();
        let diff = score - previous;
        let delta = if score.is_won_or_lost() {
            if score.is_game_won_score() { ":)" } else { ":(" }
        } else if score.0 == 0 {
            ":|"
        } else if diff >= Score(10) {
            "🡩 "
        } else if diff <= Score(-10) {
            "🡫 "
        } else if diff > Score(0) {
            "🡭 "
        } else if diff < Score(0) {
            "🡮 "
        } else {
            "🡪 "
        };
        write!(&mut res, " {}", delta.to_string().dimmed().color(TrueColor { r, g, b })).unwrap();
        res
    } else if min_width {
        res + "   "
    } else {
        res
    }
}

fn write_move_nr(res: &mut String, move_nr: usize, first_move: bool, first_player: bool, mpv_type: Option<MpvType>) {
    use fmt::Write;
    let mut write = |move_nr: String| {
        if mpv_type == Some(MainOfMultiple) {
            write!(res, "{}", move_nr.bold()).unwrap();
        } else {
            write!(res, "{}", move_nr.dimmed()).unwrap();
        }
    };
    if first_player {
        write(format!(" {move_nr}."));
    } else if first_move {
        write(format!(" {move_nr}. ..."));
    } else {
        res.push(' ');
    }
}

fn pretty_variation<B: Board>(
    pv: &[B::Move],
    mut pos: B,
    previous: Option<&[B::Move]>,
    mpv_type: Option<MpvType>,
    node_type: NodeType,
) -> (String, B) {
    use fmt::Write;
    let mut same_so_far = true;
    let mut res = String::new();
    let pv = pv.iter();
    for (idx, mov) in pv.enumerate() {
        if !pos.is_move_legal(*mov) && !mov.is_null() {
            debug_assert!(false);
            let name = if mpv_type.is_some() { "PV " } else { "" };
            return (format!("{res} [Invalid {name}move '{}']", mov.compact_formatter(&pos).to_string().red()), pos);
        }
        // 'Alternative' would be cooler, but unfortunately most fonts struggle with unicode chess pieces,
        // especially in combination with bold / dim etc
        let mut new_move = mov.to_extended_text(&pos, Standard);
        let previous = previous.and_then(|p| p.get(idx)).copied().unwrap_or_default();
        if previous == *mov || mpv_type == Some(SecondaryLine) {
            new_move = new_move.dimmed().to_string();
        } else if same_so_far && mpv_type != Some(SecondaryLine) {
            new_move = new_move.bold().to_string();
            same_so_far = false;
        }
        if node_type == FailLow {
            new_move = new_move.red().to_string();
        } else if node_type == FailHigh {
            new_move = new_move.green().to_string();
        }
        write_move_nr(&mut res, pos.fullmove_ctr_1_based(), idx == 0, pos.active_player().is_first(), mpv_type);
        write!(&mut res, "{new_move}").unwrap();
        if mov.is_null() {
            pos = pos.make_nullmove().unwrap();
        } else {
            pos = pos.make_move(*mov).unwrap();
        }
    }
    (res, pos)
}
