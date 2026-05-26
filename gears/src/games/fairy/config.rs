/*
 *  Gears, a collection of board games.
 *  Copyright (C) 2024 ToTheAnd
 *
 *  Gears is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Gears is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Gears. If not, see <https://www.gnu.org/licenses/>.
 */
//! Parsing of .ini files compatible with Fairy-Stockfsh.

// TODO: Finish implementing
#![allow(unused)]

use crate::games::fairy::attacks::GenAttackKind::DoublePawnPush;
use crate::games::fairy::attacks::{CaptureCondition, GenAttackKind, GenAttacksCondition, Modality, RequiredForAttack};
use crate::games::fairy::config::Direction::{Backwards, Forwards, Half, Left, Right, Sideways, Vertical};
use crate::games::fairy::effects::Observers;
use crate::games::fairy::piece_builder::{
    AttackBBGenBuilder, AttackKindBuilder, DropInfo, LeaperBBBuilder, PieceBuilder, RayBBBuilder, RayDir, Topology,
};
use crate::games::fairy::pieces::PieceId;
use crate::games::fairy::rules::FenHandInfo::{InBracketsEmpty, InBracketsMinusForEmpty};
use crate::games::fairy::rules::{
    CastlingInfo, DrawCtrCond, GameEndEager, GameEndRes, NoMovesCondition, Rules, RulesBuilder, RulesRef, SquareFilter,
};
use crate::games::fairy::{Bitboard, Color, RawBitboard};
use crate::games::{DimT, Height, SizeTrait, Width, char_to_file};
use crate::general::bitboards::{BitboardTrait, DynamicallySizedBitboard};
use crate::general::common::{Res, parse_bool_from_str, parse_int_from_str, tokens};
use crate::general::squares::{GridCoordinates, GridSize};
use anyhow::{anyhow, bail};
use colored::Colorize;
use configparser::ini::Ini;
use derive_more::{Display, FromStr};
use itertools::Itertools;
use std::path::Path;
use std::str::FromStr;

type OptionMap = indexmap::map::IndexMap<String, Option<String>>;

type PieceName = String;
// in theory, we could use PieceId here, but then it would be difficult and bug-prone to copy piece sets (such as promo pieces)
// to other variants.
type PieceSet = Vec<PieceName>;

fn parse_file(file: &str) -> Res<DimT> {
    if file.len() == 1 && file.chars().next().unwrap().is_ascii_alphabetic() {
        Ok(char_to_file(file.chars().next().unwrap()))
    } else {
        parse_int_from_str(file, "file")
    }
}

fn parse_square_filter(input: &str, size: GridSize) -> Res<SquareFilter> {
    let mut tokens = tokens(input);
    let mut res = RawBitboard::default();
    for word in tokens {
        if word == "*" {
            res = !0;
        } else if word.ends_with('*') {
            let file: DimT = parse_int_from_str(&word[..word.len() - 1], "file")?;
            if file == 0 {
                bail!("file must be 1-indexed and can't be 0");
            }
            res |= Bitboard::file_for(file - 1, size).raw();
        } else if word.starts_with('*') {
            let rank: DimT = parse_int_from_str(&word[1..], "rank")?;
            if rank == 0 {
                bail!("Rank must be 1-indexed and can't be 0");
            }
            res |= Bitboard::rank_for(rank - 1, size).raw();
        } else {
            let sq = GridCoordinates::from_str(word)?;
            res |= 1 << size.internal_key(sq);
        }
    }
    Ok(SquareFilter::SideRelativeBitboard(res))
}

fn parse_piece_set(input: &str, pieces: &[PieceBuilder]) -> Res<PieceSet> {
    let mut res = vec![];
    for c in input.chars() {
        let Some(piece) = pieces.iter().find(|p| p.matches_char(c)) else {
            bail!("No piece found for symbol '{}'", c.to_string().red())
        };
        res.push(piece.name.clone())
    }
    Ok(res)
}

fn parse_piece_map(input: &str, pieces: &[PieceBuilder]) -> Res<Vec<(PieceId, PieceId)>> {
    let mut res = vec![];
    for mapping in tokens(input) {
        if mapping.chars().count() != 3 || mapping.chars().nth(1) != Some(':') {
            bail!("Expected a mapping of the form '<a>:<b>', where <a> and <b> are single characters each");
        }
        let from_c = input.chars().next().unwrap();
        let Some((from, _)) = pieces.iter().find_position(|p| p.matches_char(from_c)) else {
            bail!("No piece found for symbol '{}'", from_c.to_string().red())
        };
        let to_c = input.chars().last().unwrap();
        let Some((to, _)) = pieces.iter().find_position(|p| p.matches_char(to_c)) else {
            bail!("No piece found for symbol '{}'", to_c.to_string().red())
        };
        res.push((PieceId::new(from), PieceId::new(to)))
    }
    Ok(res)
}

fn read_symbol(symbol: &str, mut piece: PieceBuilder) -> Res<PieceBuilder> {
    if symbol.is_ascii() && symbol.len() == 1 {
        piece.set_ascii_symbol(symbol.chars().next().unwrap());
        Ok(piece)
    } else {
        // TODO: Allow setting unicode symbols as well, and maybe all of the per-player and uncolered symbols
        bail!("Expected a single ascii char for the piece symbol, not '{}'", symbol.red());
    }
}

pub struct LeaperDir {
    n: usize,
    m: usize,
}

#[derive(Debug, Display, Copy, Clone, Eq, PartialEq)]
enum Atom {
    Alfil,
    Camel,
    Dabbaba,
    Ferz,
    Tripper,
    Threeleaper,
    Knight,
    Wazir,
    Zebra,
    Bishop,
    Rook,
    Queen,
    King,
}

impl Atom {
    fn from_char(c: char) -> Option<Self> {
        match c {
            'A' => Some(Self::Alfil),
            'C' => Some(Self::Camel),
            'D' => Some(Self::Dabbaba),
            'F' => Some(Self::Ferz),
            'G' => Some(Self::Tripper),
            'H' => Some(Self::Threeleaper),
            'N' => Some(Self::Knight),
            'W' => Some(Self::Wazir),
            'Z' => Some(Self::Zebra),
            'B' => Some(Self::Bishop),
            'R' => Some(Self::Rook),
            'Q' => Some(Self::Queen),
            'K' => Some(Self::King),
            // TODO: Some kind of syntax like `L(n,m)` to create an n,m leaper (see if fairy-sf has something like that).
            // Apparently part of betza 2.0
            _ => None,
        }
    }

    // n >= m
    fn leaper_n_m(self) -> Option<(isize, isize)> {
        Some(match self {
            Atom::Alfil => (2, 2),
            Atom::Camel => (3, 1),
            Atom::Dabbaba => (2, 0),
            Atom::Ferz => (1, 1),
            Atom::Tripper => (3, 3),
            Atom::Threeleaper => (3, 0),
            Atom::Knight => (2, 1),
            Atom::Wazir => (1, 0),
            Atom::Zebra => (3, 2),
            Atom::Bishop => (1, 1),
            Atom::Rook => (1, 0),
            Atom::King | Atom::Queen => {
                // The King and Queen atoms are the combination of a (1,0) and a (1, 1) leaper/rider
                return None;
            }
        })
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Direction {
    Forwards,
    Backwards,
    Left,
    Right,
    Vertical,
    Sideways,
    Half,
}

impl Direction {
    fn on_vertical_axis(self) -> bool {
        [Forwards, Backwards, Vertical].contains(&self)
    }

    fn on_horizontal_axis(self) -> bool {
        [Left, Right, Sideways].contains(&self)
    }
    fn to_ray_dirs(self) -> Vec<RayDir> {
        match self {
            Forwards => vec![RayDir { dx: 0, dy: 1 }],
            Backwards => vec![RayDir { dx: 0, dy: -1 }],
            Left => vec![RayDir { dx: -1, dy: 0 }],
            Right => vec![RayDir { dx: 1, dy: 0 }],
            Vertical => vec![RayDir { dx: 0, dy: 1 }, RayDir { dx: 0, dy: -1 }],
            Sideways => vec![RayDir { dx: -1, dy: 0 }, RayDir { dx: 1, dy: 0 }],
            Half => vec![],
        }
    }
}

#[derive(Debug)]
struct ParseAtomState {
    // Single directions like `f` are doubled (e.g. `ff`) so that every direction description consists of 2 directions
    directions: Vec<(Direction, Direction)>,
    modality: Modality,
    topology: Topology,
    atom: Atom,
    // TODO: Allow limiting rider length to `n` steps (n can be more than a single digit)
    // TODO: Also, maybe add an optional minimum number steps (not a part of betza afaik, but still useful)
    limit: Option<usize>,
}

fn parse_atom(input: &[u8], i: &mut usize) -> Res<(Atom, Option<usize>)> {
    let cur = input[*i] as char;
    let Some(piece) = Atom::from_char(cur) else { bail!("Unrecognized atom '{}'", cur.to_string().red()) };
    *i += 1;
    let next = input.get(*i).cloned().unwrap_or(b' ');
    let infinite_rider = next as char == cur || matches!(piece, Atom::Bishop | Atom::Rook | Atom::Queen);
    let limit = if next.is_ascii_digit() {
        let mut num = String::from(next as char);
        loop {
            *i += 1;
            let c = input.get(*i).cloned().unwrap_or(b' ');
            if c.is_ascii_digit() {
                num.push(c as char);
            } else {
                break;
            }
        }
        let num = parse_int_from_str(&num, "limit")?;
        if num == 0 { None } else { Some(num) }
    } else if infinite_rider {
        if next as char == cur {
            *i += 1;
        }
        None
    } else {
        Some(1)
    };
    Ok((piece, limit))
}

fn combine_dirs(dirs: Vec<Direction>, atom: Atom) -> Res<Vec<(Direction, Direction)>> {
    let mut res = vec![];
    let mut i = 0;
    while i < dirs.len() {
        let d = dirs[i];
        if d == Half {
            bail!("The half modifier '{}' must apply to a direction directly before it", "h".bold())
        }
        if i + 1 >= dirs.len() {
            res.push((d, d));
            break;
        }
        let next = dirs[i + 1];
        // Two direction modifiers combine iff they're different h/v direction or the second one is 'half' or they're the same.
        // However, for leapers with `m == 0` orthogonal directions don't combine, so there betza demands set union instead of intersection
        if (d != next && d.on_vertical_axis() == next.on_vertical_axis())
            || atom.leaper_n_m().is_some_and(|(_, m)| m == 0)
        {
            res.push((d, d));
            i += 1;
        } else {
            res.push((d, next));
            i += 2;
        }
    }
    Ok(res)
}

// parses a single atom optionally prefixed with a list of modifiers, and optionally suffixed with a range restriction
fn parse_modified_atom(input: &[u8], i: &mut usize) -> Res<ParseAtomState> {
    let mut directions = vec![];
    let mut topology = Topology::default();
    let mut modality = Modality::default();
    loop {
        if *i >= input.len() {
            bail!("Missing atom at the end of betza notation")
        }
        if input[*i].is_ascii_uppercase() {
            break;
        }
        let c = input[*i] as char;
        match c {
            'f' => directions.push(Forwards),
            'b' => directions.push(Backwards),
            'l' => directions.push(Left),
            'r' => directions.push(Right),
            'v' => directions.push(Vertical),
            's' => directions.push(Sideways),
            'h' => directions.push(Half),
            'o' => {
                if topology != Topology::default() {
                    bail!("Attempt to set the topology twice, using '{}'", "o".red())
                }
                topology = Topology::Cylinder
            }
            'm' | 'c' => {
                if modality != Modality::default() {
                    bail!(
                        "Attempt to set the modality twice: First to '{0}', then to '{1}'",
                        modality.betza_char().to_string().bold(),
                        c.to_string().red()
                    )
                }
                modality = if c == 'c' { Modality::Capture } else { Modality::NonCapture };
            }
            c if c.is_ascii_digit() => bail!("Unexpected digit '{}' must be preceded by an atom", c.to_string().red()),
            c => bail!("Unrecognized modifier '{}'", c.to_string().red()),
        }
        *i += 1;
    }
    let (atom, limit) = parse_atom(input, i)?;
    let directions = combine_dirs(directions, atom)?;
    Ok(ParseAtomState { directions, modality, topology, atom, limit })
}

fn nm_dirs_to_ray_dir(n: isize, m: isize, (a, b): (Direction, Direction)) -> Vec<RayDir> {
    let mut res = vec![];
    if [Forwards, Vertical].contains(&a) && (!b.on_horizontal_axis() || b != Right) {
        // upper left
        res.push(RayDir { dx: -m, dy: n })
    }
    if [Forwards, Vertical].contains(&a) && (!b.on_horizontal_axis() || b != Left) {
        // upper right
        res.push(RayDir { dx: m, dy: n })
    }
    if [Backwards, Vertical].contains(&a) && (!b.on_horizontal_axis() || b != Right) {
        // lower left
        res.push(RayDir { dx: -m, dy: -n })
    }
    if [Backwards, Vertical].contains(&a) && (!b.on_horizontal_axis() || b != Left) {
        // lower right
        res.push(RayDir { dx: m, dy: -n })
    }
    if [Left, Sideways].contains(&a) && (!b.on_vertical_axis() || b != Backwards) {
        // left upper
        res.push(RayDir { dx: -n, dy: m })
    }
    if [Left, Sideways].contains(&a) && (!b.on_vertical_axis() || b != Forwards) {
        // left lower
        res.push(RayDir { dx: -n, dy: -m })
    }
    if [Right, Sideways].contains(&a) && (!b.on_vertical_axis() || b != Backwards) {
        // right upper
        res.push(RayDir { dx: n, dy: m })
    }
    if [Right, Sideways].contains(&a) && (!b.on_vertical_axis() || b != Forwards) {
        // right lower
        res.push(RayDir { dx: n, dy: -m })
    }
    res
}

pub fn n_m_to_ray_dirs(n: usize, m: usize) -> Vec<RayDir> {
    let mut dirs = vec![];
    for &d in &[Vertical, Sideways] {
        dirs.extend_from_slice(&nm_dirs_to_ray_dir(n as isize, m as isize, (d, d)));
    }
    dirs.sort();
    dirs.dedup();
    dirs
}

fn make_attack_bbs(descr: &mut ParseAtomState) -> Res<AttackBBGenBuilder> {
    // There are 4 ray directions for vertical and horizontal attacks, and 8 ray directions for oblique directions (n!=m and n,m != 0).
    if descr.directions.is_empty() {
        descr.directions = vec![(Vertical, Half), (Sideways, Half)];
    }
    let mut dirs = vec![];
    if let Some((n, m)) = descr.atom.leaper_n_m() {
        for &d in &descr.directions {
            dirs.extend_from_slice(&nm_dirs_to_ray_dir(n, m, d));
        }
    } else {
        // king or queen atom, simply treat as a combination of a (1, 0) and a (1, 1) leaper
        for &d in &descr.directions {
            dirs.extend_from_slice(&nm_dirs_to_ray_dir(1, 0, d));
            dirs.extend_from_slice(&nm_dirs_to_ray_dir(1, 1, d));
        }
    }
    dirs.sort();
    dirs.dedup();
    if descr.limit == Some(1) {
        return Ok(AttackBBGenBuilder::Leaper(LeaperBBBuilder {
            offsets: dirs,
            topology: descr.topology,
            modality: descr.modality,
        }));
    } else if descr.topology == Topology::Plane && descr.limit.is_none() {
        // optimization: If the attacks match bishop/rook/queen attacks, we can select a slightly faster implementation
        let bishops =
            [RayDir { dx: -1, dy: -1 }, RayDir { dx: -1, dy: 1 }, RayDir { dx: 1, dy: -1 }, RayDir { dx: 1, dy: 1 }];
        let rooks =
            [RayDir { dx: -1, dy: 0 }, RayDir { dx: 0, dy: -1 }, RayDir { dx: 0, dy: 1 }, RayDir { dx: 1, dy: 0 }];
        if dirs == bishops {
            return Ok(AttackBBGenBuilder::PlaneBishop);
        } else if dirs == rooks {
            return Ok(AttackBBGenBuilder::PlaneRook);
        } else if dirs.len() == 8
            && bishops.iter().all(|d| dirs.contains(&d))
            && rooks.iter().all(|d| dirs.contains(&d))
        {
            return Ok(AttackBBGenBuilder::PlaneQueen);
        }
    }
    Ok(AttackBBGenBuilder::Rider(RayBBBuilder {
        ray_steps: dirs,
        limit: descr.limit,
        topology: descr.topology,
        modality: descr.modality,
    }))
}

fn make_attacks(mut descr: ParseAtomState) -> Res<AttackKindBuilder> {
    let bb_builder = make_attack_bbs(&mut descr)?;
    Ok(AttackKindBuilder {
        build_col_relative: true,
        attack_bb_gen: bb_builder,
        required: RequiredForAttack::PieceOnBoard,
        condition: GenAttacksCondition::Always,
        modality: descr.modality,
        bitboard_filter: vec![],
        kind: GenAttackKind::Normal,
        capture_condition: CaptureCondition::DestOccupied,
    })
}

fn parse_betza(input: &str) -> Res<Vec<AttackKindBuilder>> {
    if !input.is_ascii() {
        bail!("Betza piece descriptions must consist entirely of ASCII characters, but '{}' does not", input.red())
    }
    let mut res = vec![];
    let input = input.trim_ascii().as_bytes();
    let mut i = 0;
    while i < input.len() {
        let r = parse_modified_atom(input, &mut i)?;
        let attack_builder = make_attacks(r)?;
        res.push(attack_builder);
    }

    Ok(res)
}

fn modify_piece(mut piece: PieceBuilder, symbol_and_betza: &Option<String>) -> Res<Option<PieceBuilder>> {
    let Some(symbol_and_betza) = symbol_and_betza else {
        return Ok(Some(piece));
    };
    let Some((symbol, betza)) = symbol_and_betza.split_once(':') else {
        let piece = read_symbol(symbol_and_betza, piece)?;
        // as far as I can tell, `piece = -` has absolutely no effect in fairy-sf .ini files
        return if piece.uncolored_symbol[0] == '-' { Ok(None) } else { Ok(Some(piece)) };
    };
    piece = read_symbol(symbol.trim(), piece)?;
    // an empty betza notation means the piece can't move TODO: Testcase
    piece.attacks = parse_betza(betza.trim())?;
    Ok(Some(piece))
}

fn piece_from_name(name: &str) -> Option<PieceBuilder> {
    match name.to_ascii_lowercase() {
        _ => None,
    }
}

// TODO: Remove
fn not_implemented(name: &str) -> Res<()> {
    bail!("The option {} is not yet implemented", name.red())
}

fn parse_game_end(value: &str) -> Res<GameEndRes> {
    if value.eq_ignore_ascii_case("loss") {
        Ok(GameEndRes::InactivePlayerWin)
    } else if value.eq_ignore_ascii_case("win") {
        Ok(GameEndRes::ActivePlayerWin)
    } else if value.eq_ignore_ascii_case("draw") {
        Ok(GameEndRes::Draw)
    } else {
        bail!("Unrecognized game end value '{}'; should be one of ['win', 'loss', 'draw']", value.red())
    }
}

fn handle_first(map: &OptionMap, rules: &mut RulesBuilder, name: FairySFOption) -> Res<()> {
    // TODO: Impl AsRef to avoid having to construct temporary strings?
    if let Some(value) = map.get(&name.to_string().to_ascii_lowercase()) {
        let Some(value) = value else {
            bail!("Missing key for option '{}'", name.to_string().red());
        };
        apply_option(map, rules, name, &value.clone())?;
    };
    Ok(())
}

fn apply_option(map: &OptionMap, rules: &mut RulesBuilder, name: FairySFOption, value: &str) -> Res<()> {
    match name {
        FairySFOption::MaxRank => rules.size.height = Height(parse_int_from_str(value, "height")?),
        FairySFOption::MaxFile => rules.size.width = Width(parse_file(value)?),
        FairySFOption::Chess960 => todo!(),
        FairySFOption::TwoBoards => return not_implemented("twoBoards"),
        FairySFOption::StartFen => {
            rules.format_rules.startpos_fen = value.to_string();
            let pos_part = tokens(value).next().unwrap_or_default();
            if pos_part.ends_with("[-]") {
                rules.format_rules.hand = InBracketsMinusForEmpty;
            } else if pos_part.ends_with(']') {
                rules.format_rules.hand = InBracketsEmpty;
            }
        }
        // FairySFOption::MobilityRegion()
        FairySFOption::PawnTypes => todo!(),
        FairySFOption::PromotionRegionWhite | FairySFOption::PromotionRegionBlack => {
            for p in &mut rules.pieces {
                p.promotions.optional_promo_zone = parse_square_filter(value, rules.size)?;
                p.promotions.forced_promo_zone = parse_square_filter(value, rules.size)?;
                // TODO: Decide which one to set; Separate regions for white and black (keep 2 instances)
            }
        }
        FairySFOption::PromotionPawnTypes => {
            handle_first(map, rules, FairySFOption::PawnTypes)?;
            handle_first(map, rules, FairySFOption::PromotionPieceTypes)?;
            rules.pawn_info[0].promo_pieces = parse_piece_set(value, &rules.pieces)?;
            rules.pawn_info[1].promo_pieces = rules.pawn_info[0].promo_pieces.clone();
        }
        FairySFOption::PromotionPawnTypesWhite => {
            handle_first(map, rules, FairySFOption::PromotionPieceTypesWhite)?;
            handle_first(map, rules, FairySFOption::PromotionPawnTypes)?;
            return not_implemented("PromotionPawnTypesWhite");
        }
        FairySFOption::PromotionPawnTypesBlack => {
            handle_first(map, rules, FairySFOption::PromotionPieceTypesBlack)?;
            handle_first(map, rules, FairySFOption::PromotionPawnTypes)?;
            return not_implemented("PromotionPawnTypesBlack");
        }
        FairySFOption::PromotionPieceTypes => {
            handle_first(map, rules, FairySFOption::PawnTypes)?;
            for p in &mut rules.pieces {
                // TODO: is_pawn member of PieceBuilder
                // p.promotions.pieces = parse_piece_set(value)?;
            }
        }
        FairySFOption::PromotionPieceTypesWhite => {
            handle_first(map, rules, FairySFOption::PromotionPieceTypes)?;
            todo!()
        }
        FairySFOption::PromotionPieceTypesBlack => {
            handle_first(map, rules, FairySFOption::PromotionPieceTypes)?;
            todo!()
        }

        FairySFOption::PittuyinPromotion => {
            let enabled = parse_bool_from_str(value, "pittuyinPromotion")?;
            todo!();
        }

        FairySFOption::PromotionLimit => {
            let limits = parse_piece_map(value, &rules.pieces)?;
            for (piece, limit) in limits {
                let limit: usize = parse_int_from_str(value, "promotionLimit")?;
                todo!()
            }
        }
        FairySFOption::PromotedPieceType => {
            let promoted = parse_piece_map(value, &rules.pieces)?;
            for (piece, promoted) in promoted {
                rules.pieces[piece.val()].promotions.promoted_version = Some(promoted);
                rules.pieces[promoted.val()].promotions.promoted_from = Some(piece);
            }
        }
        FairySFOption::PiecePromotionOnCapture => {
            let value = parse_bool_from_str(value, "piecePromotionOnCapture")?;
            todo!();
        }
        FairySFOption::MandatoryPawnPromotion => {
            let value = parse_bool_from_str(value, "mandatoryPawnPromotion")?;
            // rules.pieces[(todo!())].promotions
        }
        FairySFOption::MandatoryPiecePromotion => {
            let value = parse_bool_from_str(value, "mandatoryPiecePromotion")?;
            for p in &mut rules.pieces {
                todo!()
            }
        }
        FairySFOption::PieceDemotion => {
            let value = parse_bool_from_str(value, "pieceDemotion")?;
            todo!()
        }
        FairySFOption::BlastOnCapture => {
            let value = parse_bool_from_str(value, "blastOnCapture")?;
            if value {
                rules.observers = Observers::atomic(todo!());
            }
        }
        FairySFOption::BlastImmuneTypes => {
            todo!()
        }
        FairySFOption::MutuallyImmuneTypes => {
            todo!()
        }
        FairySFOption::PetrifyOnCaptureTypes => {
            todo!()
        }
        FairySFOption::PetrifyBlastPieces => {
            todo!()
        }
        FairySFOption::DoubleStep => {
            // TODO: Testcase with shogi pawn and chess pawn
            let value = parse_bool_from_str(value, "doubleStep")?;
            // Enabling this option on its own does nothing, because we also need a doublestep region
            if !value {
                rules.pawn_info[0].double_steps = SquareFilter::NoSquares;
                rules.pawn_info[1].double_steps = SquareFilter::NoSquares;
            }
        }
        FairySFOption::DoubleStepRegionWhite => {
            handle_first(map, rules, FairySFOption::DoubleStep)?;
            let squares = parse_square_filter(value, rules.size)?;
            rules.pawn_info[0].double_steps = squares;
        }
        FairySFOption::DoubleStepRegionBlack => {
            handle_first(map, rules, FairySFOption::DoubleStep)?;
            let squares = parse_square_filter(value, rules.size)?;
            rules.pawn_info[1].double_steps = squares;
        }
        FairySFOption::TripleStepRegionWhite => {
            let squares = parse_square_filter(value, rules.size)?;
            rules.pawn_info[0].triple_steps = squares;
        }
        FairySFOption::TripleStepRegionBlack => {
            let squares = parse_square_filter(value, rules.size)?;
            rules.pawn_info[1].triple_steps = squares;
        }
        FairySFOption::EnPassantRegion => {
            let squares = parse_square_filter(value, rules.size)?;
            rules.pawn_info[0].ep = squares.clone();
            rules.pawn_info[1].ep = squares;
        }
        FairySFOption::EnPassantRegionWhite => {
            handle_first(map, rules, FairySFOption::EnPassantRegion)?;
            let squares = parse_square_filter(value, rules.size)?;
            rules.pawn_info[0].triple_steps = squares;
        }
        FairySFOption::EnPassantRegionBlack => {
            handle_first(map, rules, FairySFOption::EnPassantRegion)?;
            let squares = parse_square_filter(value, rules.size)?;
            rules.pawn_info[1].triple_steps = squares;
        }
        FairySFOption::EnPassantTypes => {
            let pieces = parse_piece_set(value, &rules.pieces)?;
            rules.pawn_info[0].ep_types = pieces.clone();
            rules.pawn_info[1].ep_types = pieces;
        }
        FairySFOption::EnPassantTypesWhite => {
            handle_first(map, rules, FairySFOption::EnPassantTypes)?;
            let pieces = parse_piece_set(value, &rules.pieces)?;
            rules.pawn_info[0].ep_types = pieces.clone();
        }
        FairySFOption::EnPassantTypesBlack => {
            handle_first(map, rules, FairySFOption::EnPassantTypes)?;
            let pieces = parse_piece_set(value, &rules.pieces)?;
            rules.pawn_info[1].ep_types = pieces.clone();
        }
        FairySFOption::Castling => {
            let enabled = parse_bool_from_str(value, "castling")?;
            if rules.castling.is_none() && enabled {
                rules.castling = Some(CastlingInfo::new([2, rules.size.width.0 - 2]))
            } else if !enabled {
                rules.castling = None;
            }
        }
        FairySFOption::PieceDrops => {
            let val = parse_bool_from_str(value, "pieceDrops")?;
            for p in &mut rules.pieces {
                p.drop_info = val.then_some(DropInfo::default());
            }
        }
        FairySFOption::CapturesToHand => {
            let val = parse_bool_from_str(value, "captures to hand")?;
            if val {
                rules.observers.capture.push(Observers::add_captured_to_hand());
            }
            // TODO: Unset if false. Probably better to not use observers, but instead to hard-code this
        }
        FairySFOption::DropNoDoubled => {
            let pieces = parse_piece_set(value, &rules.pieces)?;
            for p in &mut rules.pieces {
                let Some(info) = &mut p.drop_info else {
                    bail!("Cannot set '{0}' when '{1}' is false", "dropNoDouble".bold(), "pieceDrops".red())
                };
                info.drop_no_double = false;
            }
            for p in pieces {
                let Some(piece) = rules.pieces.iter_mut().find(|piece| piece.name == p) else {
                    bail!("Piece '{}' doesn't exist in the current game", p.red());
                };
                let info = piece.drop_info.as_mut().unwrap();
                info.drop_no_double = true;
            }
        }
        FairySFOption::ImmobilityIllegal => {
            let val = parse_bool_from_str(value, "immobilityIllegal")?;
            rules.immobility_illegal = val;
        }
        FairySFOption::NMoveRule => {
            let val: usize = parse_int_from_str(value, "nmove")?;
            rules.game_end_eager.retain(|(c, _)| matches!(c, GameEndEager::DrawCounter(_)));
            if val > 0 {
                rules.game_end_eager.push((GameEndEager::DrawCounter(DrawCtrCond::Fixed(val * 2)), GameEndRes::Draw));
            }
        }
        FairySFOption::NFoldRule => {
            let val = parse_int_from_str(value, "nfold")?;
            for (c, r) in &mut rules.game_end_eager {
                if let GameEndEager::Repetition(n) = c {
                    *n = val;
                    return Ok(());
                }
            }
            rules.game_end_eager.push((GameEndEager::Repetition(val), GameEndRes::Draw));
        }
        FairySFOption::NFoldValue => {
            let res = parse_game_end(value)?;
            for (c, r) in &mut rules.game_end_eager {
                if let GameEndEager::Repetition(_) = c {
                    *r = res;
                    return Ok(());
                }
            }
            rules.game_end_eager.push((GameEndEager::Repetition(3), res));
        }
        FairySFOption::NFoldValueAbsolute => {
            () = handle_first(map, rules, FairySFOption::NFoldValue)?;
            let val = parse_bool_from_str(value, "nfold value absolute")?;
            for (c, r) in &mut rules.game_end_eager {
                if let GameEndEager::Repetition(_) = c {
                    match *r {
                        GameEndRes::ActivePlayerWin => *r = GameEndRes::FirstPlayerWin,
                        GameEndRes::InactivePlayerWin => *r = GameEndRes::SecondPlayerWin,
                        _ => {}
                    };
                    return Ok(());
                }
            }
        }
        FairySFOption::PerpetualCheckIllegal => {
            let val = parse_bool_from_str(value, "perpetualCheckIllegal")?;
            if val {
                rules.check_rules.perpetual_is_loss = true; // TODO: Actually handle this option
            }
        }
        FairySFOption::StalemateValue => {
            let game_res = parse_game_end(value)?;
            rules.game_end_no_moves.retain(|&(c, _)| c != NoMovesCondition::NotInCheck);
            rules.game_end_no_moves.push((NoMovesCondition::NotInCheck, game_res));
        }
        FairySFOption::CheckmateValue => {
            let game_res = parse_game_end(value)?;
            rules.game_end_no_moves.retain(|&(c, _)| c != NoMovesCondition::InCheck);
            rules.game_end_no_moves.push((NoMovesCondition::InCheck, game_res));
        }
        FairySFOption::ShogiPawnDropMateIllegal => {
            let val = parse_bool_from_str(value, "shogiPawnDropMateIllegal")?;
            if val {
                rules.observers.finish_move.push(Observers::shogi_pawn_mate_drop_illegal());
            }
            // TODO: Allow unsetting
        }
        FairySFOption::VariantTemplate | FairySFOption::PocketSize => { /*ignored*/ }
    }
    Ok(())
}

enum OptionValue {
    Rank(usize),
    File(usize),
    Bool(bool),
    Squares(SquareFilter),
    Pieces(PieceSet),
}

// See <https://github.com/fairy-stockfish/Fairy-Stockfish/blob/master/src/variants.ini>
#[derive(Debug, Copy, Clone, Eq, PartialEq, Display, FromStr)]
enum FairySFOption {
    MaxRank,
    MaxFile,
    Chess960,
    TwoBoards,
    StartFen,
    // MobilityRegion(todo!()),
    PawnTypes,
    PromotionRegionWhite,
    PromotionRegionBlack,
    PromotionPawnTypes,
    PromotionPawnTypesWhite,
    PromotionPawnTypesBlack,
    PromotionPieceTypes,
    PromotionPieceTypesWhite,
    PromotionPieceTypesBlack,
    PittuyinPromotion,
    PromotionLimit,
    PromotedPieceType,
    PiecePromotionOnCapture,
    MandatoryPawnPromotion,
    MandatoryPiecePromotion,
    PieceDemotion,
    BlastOnCapture,
    BlastImmuneTypes,
    MutuallyImmuneTypes,
    PetrifyOnCaptureTypes,
    PetrifyBlastPieces,
    DoubleStep,
    DoubleStepRegionWhite,
    DoubleStepRegionBlack,
    TripleStepRegionWhite,
    TripleStepRegionBlack,
    EnPassantRegion,
    EnPassantRegionWhite,
    EnPassantRegionBlack,
    EnPassantTypes,
    EnPassantTypesWhite,
    EnPassantTypesBlack,
    Castling,
    //     CastlingDroppedPiece,
    //     CastlingKingsideFile(SquareFilter),
    //     CastlingQueensideFile(SquareFilter),
    //     CastlingRank(SquareFilter),
    //     CastlingKingFile(SquareFilter),
    //     CastlingKingPiece(PieceName),
    //     CastlingRookKingsideFile(SquareFilter),
    //     CastlingRookQueensideFile(SquareFilter),
    //     CastlingRookPieces(PieceName),
    //     OppositeCastling,
    //     Checking,
    //     DropChecks,
    //     MustCapture,
    //     MustDrop,
    //     MustDropType(PieceSet),
    PieceDrops,
    // # dropLoop: captures promoted pieces are not demoted [bool] (default: false)
    CapturesToHand,
    // # firstRankPawnDrops: allow pawn drops to first rank [bool] (default: false)
    // # promotionZonePawnDrops: allow pawn drops in promotion zone  [bool] (default: false)
    // # enclosingDrop: require piece drop to enclose pieces [EnclosingRule] (default: none)
    // # enclosingDropStart: drop region for starting phase disregarding enclosingDrop (e.g., for reversi) [Bitboard]
    // # dropRegionWhite: restrict region for piece drops of all white pieces [Bitboard]
    // # dropRegionBlack: restrict region for piece drops of all black pieces [Bitboard]
    // # sittuyinRookDrop: restrict region of rook drops to first rank [bool] (default: false)
    // # dropOppositeColoredBishop: dropped bishops have to be on opposite-colored squares [bool] (default: false)
    // # dropPromoted: pieces may be dropped in promoted state [bool] (default: false)
    DropNoDoubled,
    // # dropNoDoubledCount: specifies the count of already existing pieces for dropNoDoubled [int] (default: 1)
    ImmobilityIllegal,
    // # gating: maintain squares on backrank with extra rights in castling field of FEN [bool] (default: false)
    // # wallingRule: rule on where wall can be placed [WallingRule] (default: none)
    // # wallingRegionWhite: mask where wall squares (including duck) can be placed by white [Bitboard] (default: all squares)
    // # wallingRegionBlack: mask where wall squares (including duck) can be placed by black [Bitboard] (default: all squares)
    // # wallOrMove: can wall or move, but not both [bool] (default: false)
    // # seirawanGating: allow gating of pieces in hand like in S-Chess, requires "gating = true" [bool] (default: false)
    // # cambodianMoves: enable special moves of cambodian chess, requires "gating = true" [bool] (default: false)
    // # diagonalLines: enable special moves along diagonal for specific squares (Janggi) [Bitboard]
    // # pass: allow passing [bool] (default: false)
    // # passWhite: allow passing for white [bool] (default: false)
    // # passBlack: allow passing for black [bool] (default: false)
    // # passOnStalemate: allow passing in case of stalemate [bool] (default: false)
    // # passOnStalemateWhite: allow passing in case of stalemate for white [bool] (default: false)
    // # passOnStalemateBlack: allow passing in case of stalemate for black [bool] (default: false)
    // # makpongRule: the king may not move away from check [bool] (default: false)
    // # flyingGeneral: disallow general face-off like in xiangqi [bool] (default: false)
    // # soldierPromotionRank: restrict soldier to shogi pawn movements until reaching n-th rank [Rank] (default: 1)
    // # flipEnclosedPieces: change color of pieces that are enclosed by a drop [EnclosingRule] (default: none)
    // # nMoveRuleTypes: define pieces resetting n move rule on irreversible moves [PieceSet] (default: p)
    // # nMoveRuleTypesWhite: define white pieces resetting n move rule on irreversible moves [PieceSet] (default: p)
    // # nMoveRuleTypesBlack: define black pieces resetting n move rule on irreversible moves [PieceSet] (default: p)
    NMoveRule,
    NFoldRule,
    NFoldValue,
    NFoldValueAbsolute,
    PerpetualCheckIllegal, // despite the name, perpetuals aren't illegal, just automatic losses
    // # moveRepetitionIllegal: prohibit moving back and forth with the same piece nFoldRule-1 times [bool] (default: false)
    // # chasingRule: enable chasing rules [ChasingRule] (default: none)
    StalemateValue,
    // # stalematePieceCount: count material in case of stalemate [bool] (default: false)
    CheckmateValue,
    ShogiPawnDropMateIllegal,
    // # shatarMateRule: enable shatar mating rules [bool] (default: false)
    // # bikjangRule: consider Janggi bikjang (facing kings) rule [bool] (default: false)
    // # extinctionValue: result when one of extinctionPieceTypes is extinct [Value] (default: none)
    // # extinctionClaim: extinction of opponent pieces can only be claimed as side to move [bool] (default: false)
    // # extinctionPseudoRoyal: treat the last extinction piece like a royal piece [bool] (default: false)
    // # dupleCheck: when all pseudo-royal pieces are attacked, it counts as a check [bool] (default: false)
    // # extinctionPieceTypes: list of piece types for extinction rules, e.g., pnbrq (* means all) (default: )
    // # extinctionPieceCount: piece count at which the game is decided by extinction rule (default: 0)
    // # extinctionOpponentPieceCount: opponent piece count required to adjudicate by extinction rule (default: 0)
    // # flagPiece: piece type for capture the flag win rule [PieceType] (default: *)
    // # flagPieceWhite: piece type for capture the flag win rule [PieceType] (default: *)
    // # flagPieceBlack: piece type for capture the flag win rule [PieceType] (default: *)
    // # flagRegion: target region for capture the flag win rule [Bitboard] (default: )
    // # flagRegionWhite: white's target region for capture the flag win rule [Bitboard] (default: )
    // # flagRegionBlack: black's target region for capture the flag win rule [Bitboard] (default: )
    // # flagPieceCount: number of flag pieces that have to be in the flag zone [int] (default: 1)
    // # flagPieceBlockedWin: for flagPieceCount > 1, win if at least one flag piece in flag zone and all others occupied by pieces [bool] (default: false)
    // # flagMove: the other side gets one more move after one reaches the flag zone [bool] (default: false)
    // # flagPieceSafe: the flag piece must be safe to win [bool] (default: false)
    // # checkCounting: enable check count win rule (check count is communicated via FEN, see 3check) [bool] (default: false)
    // # connectN: number of aligned pieces for win [int] (default: 0)
    // # connectPieceTypes: pieces evaluated for connection rule [PieceSet] (default: *)
    // # connectVertical: connectN looks at Vertical rows [bool] (default: true)
    // # connectHorizontal: connectN looks at Horizontal rows [bool] (default: true)
    // # connectDiagonal: connectN looks at Diagonal rows [bool] (default: true)
    // # connectRegion1White: connect Region 1 to Region 2 for win. obeys connectVertical, connectHorizontal, connectDiagonal [Bitboard] (default: -)
    // # connectRegion2White: "
    // # connectRegion1Black: "
    // # connectRegion2Black: "
    // # connectNxN: connect a tight NxN square for win [int] (default: 0)
    // # collinearN: arrange N pieces collinearly (other squares can be between pieces) [int] (default: 0)
    // # connectValue: result in case of connect [Value] (default: win)
    // # materialCounting: enable material counting rules [MaterialCounting] (default: none)
    // # adjudicateFullBoard: apply material counting immediately when board is full [bool] (default: false)
    // # countingRule: enable counting rules [CountingRule] (default: none)
    // # castlingWins: Specified castling moves are win conditions. Losing these rights is losing. [CastlingRights] (default: -)
    VariantTemplate,
    PocketSize,
}

#[derive(Debug)]
pub struct GameConfig {
    pub name: String,
    pub base: Option<String>,
    pub definition: OptionMap,
}

impl GameConfig {
    fn base(&self, all: &[Self]) -> Res<RulesBuilder> {
        let Some(base) = self.base.clone() else {
            let mut res = RulesBuilder::chess();
            res.pieces.clear();
            res.has_ep = false;
            return Ok(res);
        };
        let Some(base) = all.iter().find(|config| config.name.eq_ignore_ascii_case(&base)) else {
            bail!("Base variant '{0}' does not exist for variant '{1}'", base.red(), self.name.bold())
        };
        base.create_builder(all)
    }

    fn create_builder(&self, all: &[Self]) -> Res<RulesBuilder> {
        let mut rules = self.base(all)?;
        rules.name = self.name.clone();
        let all_pieces = PieceBuilder::complete_piece_map();
        for (key_string, value) in &self.definition {
            if let Ok(key) = FairySFOption::from_str(&key_string) {
                let Some(value) = value else {
                    bail!("Missing value for configuration key '{}'", key.to_string().bold());
                };
                apply_option(&self.definition, &mut rules, key, value)?;
                continue;
            }
            // if a key is in the piece map, it refers to that piece
            if let Some(piece) = all_pieces.get(key_string) {
                if let Some(piece) = modify_piece(piece.clone(), value)? {
                    rules.pieces.push(piece);
                }
                continue;
            }
            // else, it might be a predefined name
            bail!(
                "Unrecognized configuration key '{}' (not all fairy-sf configuration keys are currently implemented)",
                key_string.red()
            );
        }
        Ok(rules)
    }

    fn create(&self, all: &[Self]) -> Res<RulesRef> {
        let rules = self.create_builder(all)?;
        let rules = rules.build();
        Ok(RulesRef::new(rules))
    }
}

fn create_configs(map: indexmap::map::IndexMap<String, OptionMap>) -> Res<Vec<GameConfig>> {
    let mut res = vec![];
    for (mut name, definition) in map {
        let mut base = None;
        if let Some((n, b)) = name.split_once(':') {
            let b = b.trim();
            if b.is_empty() {
                bail!(
                    "Variant base name of '{0}' can't be the empty string {1}; remove trailing ':'",
                    n.bold(),
                    "''".red()
                )
            }
            base = Some(b.trim().to_string());
            name = n.trim().to_string();
        }
        if name.trim().is_empty() {
            bail!("Variant name can't be empty ('{name}')");
        }
        let name = name.trim().to_string();
        res.push(GameConfig { name, base, definition });
    }
    Ok(res)
}

fn read_config_from_string(config: String) -> Res<Vec<GameConfig>> {
    let mut c = Ini::new_cs();
    let map = c.read(config).map_err(|e| anyhow!("Couldn't read the config string: {e}"))?;
    create_configs(map)
}

fn read_config(file: &Path) -> Res<Vec<GameConfig>> {
    let mut config = Ini::new();
    let map = config.load(file).map_err(|e| anyhow!("Couldn't load the config file: {e}"))?;
    create_configs(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::fairy::Board;
    use crate::general::board::{BoardHelpers, BoardTrait};
    use crate::general::squares::GridSize;

    #[test]
    fn simple_betza_atom_test() {
        for input in ["F", "Q", "F0", "AA", "A123"] {
            let attacks = parse_betza(input).unwrap();
            assert_eq!(attacks.len(), 1);
            let a = attacks[0].clone();
            assert_eq!(a.modality, Modality::Both);
            assert_eq!(a.condition, GenAttacksCondition::Always);
            assert_eq!(a.kind, GenAttackKind::Normal);
            assert_eq!(a.required, RequiredForAttack::PieceOnBoard);
            assert!(a.build_col_relative);
            assert!(a.bitboard_filter.is_empty());
            if input == "AA" || input == "A123" {
                let AttackBBGenBuilder::Rider(ray_builder) = a.attack_bb_gen else { unreachable!() };
                assert_eq!(ray_builder.modality, Modality::Both);
                assert_eq!(ray_builder.topology, Topology::Plane);
                if input == "A123" {
                    assert_eq!(ray_builder.limit, Some(123));
                } else {
                    assert_eq!(ray_builder.limit, None);
                }
                assert_eq!(ray_builder.ray_steps.len(), 4);
                assert_eq!(
                    ray_builder.ray_steps,
                    vec![
                        RayDir { dx: -2, dy: -2 },
                        RayDir { dx: -2, dy: 2 },
                        RayDir { dx: 2, dy: -2 },
                        RayDir { dx: 2, dy: 2 }
                    ]
                );
            } else if input == "F0" {
                assert!(matches!(a.attack_bb_gen, AttackBBGenBuilder::PlaneBishop));
            } else if input == "Q" {
                assert!(matches!(a.attack_bb_gen, AttackBBGenBuilder::PlaneQueen));
            } else {
                let AttackBBGenBuilder::Leaper(leaper_builder) = a.attack_bb_gen else { unreachable!() };
                assert_eq!(leaper_builder.modality, Modality::Both);
                assert_eq!(leaper_builder.topology, Topology::Plane);
                assert_eq!(leaper_builder.offsets.len(), 4);
                assert_eq!(
                    leaper_builder.offsets,
                    vec![
                        RayDir { dx: -1, dy: -1 },
                        RayDir { dx: -1, dy: 1 },
                        RayDir { dx: 1, dy: -1 },
                        RayDir { dx: 1, dy: 1 }
                    ]
                )
            }
        }

        assert_eq!(parse_betza("Q").unwrap(), parse_betza("KK").unwrap());
        assert_eq!(parse_betza("R").unwrap(), parse_betza("W0").unwrap());
    }

    #[test]
    fn parse_betza_test() {
        let input = "ffrrcN";
        let attacks = parse_betza(input).unwrap();
        assert_eq!(attacks.len(), 1);
        let a = attacks[0].clone();
        let AttackBBGenBuilder::Leaper(b) = a.attack_bb_gen else { unreachable!() };
        assert_eq!(b.modality, Modality::Capture);
        assert_eq!(
            b.offsets,
            vec![RayDir { dx: -1, dy: 2 }, RayDir { dx: 1, dy: 2 }, RayDir { dx: 2, dy: -1 }, RayDir { dx: 2, dy: 1 }]
        );

        let input = "frN";
        let attacks = parse_betza(input).unwrap();
        assert_eq!(attacks.len(), 1);
        let a = attacks[0].clone();
        let AttackBBGenBuilder::Leaper(b) = a.attack_bb_gen else { unreachable!() };
        assert_eq!(b.offsets, vec![RayDir { dx: 1, dy: 2 }]);

        let input = "frW";
        let attacks = parse_betza(input).unwrap();
        assert_eq!(attacks.len(), 1);
        let a = attacks[0].clone();
        let AttackBBGenBuilder::Leaper(b) = a.attack_bb_gen else { unreachable!() };
        assert_eq!(b.offsets, vec![RayDir { dx: 0, dy: 1 }, RayDir { dx: 1, dy: 0 }]);

        let input = "fmWfcF"; // shatranj pawn
        let attacks = parse_betza(input).unwrap();
        assert_eq!(attacks.len(), 2);
        let a0 = attacks[0].clone();
        let AttackBBGenBuilder::Leaper(b) = a0.attack_bb_gen else { unreachable!() };
        assert_eq!(b.modality, Modality::NonCapture);
        assert_eq!(b.offsets, vec![RayDir { dx: 0, dy: 1 }]);
        let a1 = attacks[1].clone();
        let AttackBBGenBuilder::Leaper(b) = a1.attack_bb_gen else { unreachable!() };
        assert_eq!(b.modality, Modality::Capture);
        assert_eq!(b.offsets, vec![RayDir { dx: -1, dy: 1 }, RayDir { dx: 1, dy: 1 }]);
    }

    #[test]
    fn parse_minishogi_config_test() {
        let config = r#"
            [minishogi]
            variantTemplate = shogi
            maxRank = 5
            maxFile = 5
            shogiPawn = p
            silver = s
            gold = g
            bishop = b
            dragonHorse = h
            rook = r
            bers = d
            king = k
            startFen = rbsgk/4p/5/P4/KGSBR[-] w 0 1
            pieceDrops = true
            capturesToHand = true
            promotionRegionWhite = *5
            promotionRegionBlack = *1
            doubleStep = false
            castling = false
            promotedPieceType = p:g s:g b:h r:d
            dropNoDoubled = p
            immobilityIllegal = true
            shogiPawnDropMateIllegal = true
            stalemateValue = loss
            nFoldRule = 4
            nMoveRule = 0
            perpetualCheckIllegal = true
            pocketSize = 5
            nFoldValue = loss
            nFoldValueAbsolute = true
            "#;
        let config = read_config_from_string(config.to_string()).unwrap();
        assert_eq!(config.len(), 1);
        let config = &config[0];
        assert_eq!(config.name, "minishogi");
        assert_eq!(config.base, None);
        assert_eq!(config.definition.len(), 29);
        let fen = "rbsgk/4p/5/P4/KGSBR[-] w 0 1".to_string();
        assert_eq!(config.definition.get("startFen"), Some(&Some(fen.clone())));
        let rules = config.create(&[]).unwrap();
        let v = rules.get();
        assert_eq!(v.size, GridSize::new(Height::new(5), Width::new(5)));
        assert_eq!(v.pieces.len(), 8);
        assert_eq!(v.name, config.name);
        let pos = Board::startpos_for_settings(rules);
        assert_eq!(pos.fen_no_rules(), fen);
        assert_eq!(pos.num_legal_moves(), 14);
    }
}
