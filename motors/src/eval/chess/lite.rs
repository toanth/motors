use std::fmt::Display;

use crate::eval::chess::lite_values::*;
use crate::eval::chess::{
    pawn_advanced_center_idx, pawn_passive_center_idx, pawn_shield_idx, DiagonalOpenness, FileOpenness, FLANK,
    REACHABLE_PAWNS,
};
use gears::games::chess::castling::CastleRight::Kingside;
use gears::games::chess::moves::Move;
use gears::games::chess::pieces::PieceType;
use gears::games::chess::pieces::PieceType::*;
use gears::games::chess::see::SeeScore;
use gears::games::chess::squares::{ChessboardSize, Square};
use gears::games::chess::Color::{Black, White};
use gears::games::chess::{Board, ChessBitboardTrait, Color, CHESS_PIECE_PHASE};
use gears::games::{ColorTrait, CoordinatesTrait};
use gears::games::{DimT, PosHash};
use gears::general::attacks::ChessSliderGenerator;
use gears::general::bitboards::chessboard::{dark_squares, light_squares, Bitboard, COLORED_SQUARES};
use gears::general::bitboards::RawBitboardTrait;
use gears::general::bitboards::{BitboardTrait, KnownSizeBitboard};
use gears::general::board::{BitboardBoard, BoardTrait};
use gears::general::common::StaticallyNamedEntity;
use gears::general::moves::MoveTrait;
use gears::general::squares::RectangularCoordinates;
use gears::score::{PhaseType, PhasedScore, Score, ScoreT};

use crate::eval::chess::king_gambot::KingGambotValues;
use crate::eval::chess::lite::FileOpenness::{Closed, Open, SemiClosed, SemiOpen};
use crate::eval::{Eval, ScoreType, SingleFeatureScore};
use crate::spsa_params;

#[derive(Debug, Default, Copy, Clone)]
struct EvalState<Tuned: LiteValues> {
    hash: PosHash,
    pawn_key: PosHash,
    passers: Bitboard,
    phase: PhaseType,
    // scores are stored from the perspective of the white player
    material: Tuned::Score,
    psqt_score: Tuned::Score,
    pawn_score: Tuned::Score,
    stm_bonus: [Tuned::Score; 2],
    total_score: Tuned::Score,
}

const STACK_SIZE: usize = 512;

#[derive(Debug, Clone)]
pub struct GenericLiTEval<Tuned: LiteValues> {
    stack: Vec<EvalState<Tuned>>,
    tuned: Tuned,
}

impl<Tuned: LiteValues> Default for GenericLiTEval<Tuned> {
    fn default() -> Self {
        Self { stack: vec![EvalState::default(); STACK_SIZE], tuned: Tuned::default() }
    }
}

pub type LiTEval = GenericLiTEval<Lite>;

pub type KingGambot = GenericLiTEval<KingGambotValues>;

spsa_params![lc,
    tempo: ScoreT = 10; 0..=32; step=1;
];

pub const TEMPO: Score = Score(lc::tempo());
// TODO: Differentiate between rooks and kings in front of / behind pawns?

fn openness(ray: Bitboard, our_pawns: Bitboard, their_pawns: Bitboard) -> FileOpenness {
    if !ray.intersects(our_pawns) && !ray.intersects(their_pawns) {
        Open
    } else if !ray.intersects(our_pawns) {
        SemiOpen
    } else if ray.intersects(our_pawns) && ray.intersects(their_pawns) {
        Closed
    } else {
        SemiClosed
    }
}

pub fn file_openness(file: DimT, our_pawns: Bitboard, their_pawns: Bitboard) -> FileOpenness {
    let file = Bitboard::file(file);
    openness(file, our_pawns, their_pawns)
}

pub fn diagonal_openness(square: Square, our_pawns: Bitboard, their_pawns: Bitboard) -> (DiagonalOpenness, usize) {
    // TODO: don't pass size
    let diag = Bitboard::diag_for_sq(square, ChessboardSize::default());
    (openness(diag, our_pawns, their_pawns), diag.num_ones())
}

pub fn anti_diagonal_openness(square: Square, our_pawns: Bitboard, their_pawns: Bitboard) -> (DiagonalOpenness, usize) {
    let anti_diag = Bitboard::anti_diag_for_sq(square, ChessboardSize::default());
    (openness(anti_diag, our_pawns, their_pawns), anti_diag.num_ones())
}

impl<Tuned: LiteValues> StaticallyNamedEntity for GenericLiTEval<Tuned> {
    fn static_short_name() -> impl Display
    where
        Self: Sized,
    {
        Tuned::static_short_name()
    }

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        Tuned::static_long_name()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        Tuned::static_description()
    }
}

impl<Tuned: LiteValues> GenericLiTEval<Tuned> {
    fn psqt(&self, pos: &Board) -> (Tuned::Score, Tuned::Score) {
        let mut psqt = Tuned::Score::default();
        let mut material = Tuned::Score::default();
        for color in Color::iter() {
            let flip = if pos.king_sq(color).file() < 4 { 0x7 } else { 0x0 };
            for piece in PieceType::pieces() {
                for square in pos.col_piece_bb(color, piece) {
                    psqt += self.tuned.psqt(Square::from_bb_idx(square.bb_idx() ^ flip), piece, color);
                }
                material += Tuned::material(piece) * pos.col_piece_bb(color, piece).num_ones();
            }
            psqt = -psqt;
            material = -material;
        }
        (material, psqt)
    }

    fn bishop_pair(pos: &Board, color: Color) -> SingleFeatureScore<Tuned::Score> {
        if pos.col_piece_bb(color, Bishop).more_than_one_bit_set() { Tuned::bishop_pair() } else { Default::default() }
    }

    fn bad_bishop(pos: &Board, color: Color) -> Tuned::Score {
        let mut score = Tuned::Score::default();
        let pawns = pos.col_piece_bb(color, Pawn);
        for bishop in pos.col_piece_bb(color, Bishop) {
            let sq_color = bishop.square_color();
            score += Tuned::bad_bishop((COLORED_SQUARES[sq_color as usize] & pawns).num_ones().min(8));
        }
        score
    }

    fn pawn_shield_for(pos: &Board, color: Color) -> SingleFeatureScore<Tuned::Score> {
        let our_pawns = pos.col_piece_bb(color, Pawn);
        let king_square = pos.king_sq(color);
        let idx = pawn_shield_idx(our_pawns, king_square, color);
        Tuned::default().pawn_shield(color, idx)
    }

    fn pawn_center(pos: &Board) -> Tuned::Score {
        let mut score = Tuned::Score::default();
        for color in Color::iter() {
            let advanced_idx = pawn_advanced_center_idx(pos.col_piece_bb(color, Pawn), color);
            let passive_idx = pawn_passive_center_idx(pos.col_piece_bb(color, Pawn), color);
            score += Tuned::pawn_advanced_center(advanced_idx);
            score += Tuned::pawn_passive_center(passive_idx);
            score = -score;
        }
        score
    }

    fn pawns_for(pos: &Board, us: Color, passers: &mut Bitboard) -> Tuned::Score {
        let our_pawns = pos.col_piece_bb(us, Pawn);
        let their_pawns = pos.col_piece_bb(us.other(), Pawn);
        let all_pawns = pos.piece_bb(Pawn);
        let mut score = Tuned::Score::default();
        score += Self::pawn_shield_for(pos, us);
        let our_king = pos.king_sq(us);
        // Idea from Stockfish
        if (all_pawns & FLANK[our_king.file() as usize]).is_zero() {
            score += Tuned::pawnless_flank();
        }
        for square in our_pawns {
            let normalized_square = square.flip_if(us == Black);
            let in_front = (Bitboard::A_FILE << (square.flip_if(us == Black).bb_idx() + 8)).flip_if(us == Black);
            let blocking_squares = in_front | in_front.west() | in_front.east();
            let file = Bitboard::file(square.file());
            let neighbor_files = file.west() | file.east();
            let supporting = neighbor_files & !blocking_squares;
            if (supporting & our_pawns).is_zero() {
                score += Tuned::unsupported_pawn();
            }
            // passed pawn
            if (in_front & our_pawns).is_zero() && (blocking_squares & their_pawns).is_zero() {
                let mirrored_sq = if our_king.file() < 4 {
                    normalized_square.flip_left_right(ChessboardSize::default())
                } else {
                    normalized_square
                };
                score += Tuned::passed_pawn(mirrored_sq);
                let their_king = pos.king_sq(!us).flip_if(us == Black);
                if REACHABLE_PAWNS[their_king.bb_idx()].has(normalized_square) {
                    score += Tuned::stoppable_passer();
                }
                let near_king = Board::normal_king_attacks_from(square) & Board::normal_king_attacks_from(our_king);
                if near_king.has_any() {
                    score += Tuned::close_king_passer();
                }
                if pos.player_bb(!us).has(square.pawn_advance_unchecked(us)) {
                    score += Tuned::immobile_passer()
                }
                *passers |= square.bb();
            }
            // may become a passer
            if (in_front & all_pawns).is_zero()
                && (blocking_squares & their_pawns).num_ones() <= (supporting & our_pawns).num_ones()
            {
                score += Tuned::candidate_passer(normalized_square.rank() - 1);
            }
            let sq_bb = square.bb();
            if our_pawns.intersects(sq_bb.east() | sq_bb.west()) {
                score += Tuned::phalanx(normalized_square.rank() - 1);
            }
        }
        let num_doubled_pawns = (our_pawns & (our_pawns.north())).num_ones();
        score += Tuned::doubled_pawn() * num_doubled_pawns;
        score
    }

    fn pawns(pos: &Board, passers: &mut Bitboard) -> Tuned::Score {
        *passers = Bitboard::default();
        Self::pawn_center(pos) + Self::pawns_for(pos, White, passers) - Self::pawns_for(pos, Black, passers)
    }

    fn useless_material_advantage(state: &mut EvalState<Tuned>, pos: &Board) -> Tuned::Score {
        if state.material == Tuned::Score::default() {
            return Tuned::Score::default();
        }
        let white_advantage = pos.material_advantage_of(White);
        let mut res = Tuned::Score::default();
        let c = if white_advantage > SeeScore(0) { White } else { Black };
        if pos.col_piece_bb(c, Pawn).is_zero() {
            let minor_pieces = pos.piece_bb(Knight) | pos.piece_bb(Bishop);
            if (minor_pieces & pos.player_bb(c)).num_ones() * 2 > minor_pieces.num_ones() {
                res += Tuned::more_minors_but_no_pawns();
            }
        }
        let white_bishops = pos.col_piece_bb(White, Bishop);
        let black_bishops = pos.col_piece_bb(Black, Bishop);
        let major = pos.piece_bb(Rook) | pos.piece_bb(Queen);
        if
        /* !pos.piece_bb(Queen).has_any()
        && pos.col_piece_bb(White, Rook).num_ones() == pos.col_piece_bb(Black, Rook).num_ones()
        && pos.col_piece_bb(White, Knight).num_ones() == pos.col_piece_bb(Black, Knight).num_ones()
        &&*/
        (major & pos.player_bb(White)).num_ones() == (major & pos.player_bb(Black)).num_ones()
            && pos.col_piece_bb(c, Pawn).has_any()
            && white_bishops.has_any()
            && black_bishops.has_any()
            && ((light_squares().contains(white_bishops) && dark_squares().contains(black_bishops))
                || (light_squares().contains(black_bishops) && dark_squares().contains(white_bishops)))
        {
            res += Tuned::opposite_colored_bishops();
        }

        if white_advantage > SeeScore(0) { res } else { -res }
    }

    fn open_lines(pos: &Board, color: Color) -> Tuned::Score {
        let mut score = Tuned::Score::default();
        let our_pawns = pos.col_piece_bb(color, Pawn);
        let their_pawns = pos.col_piece_bb(color.other(), Pawn);
        // Rooks on (semi)open/closed files (semi-closed files are handled by adjusting the base rook values during tuning)
        let rooks = pos.col_piece_bb(color, Rook);
        for rook in rooks {
            score += Tuned::rook_openness(file_openness(rook.file(), our_pawns, their_pawns));
        }
        // King on (semi)open/closed file
        let king_square = pos.king_sq(color);
        let king_file = king_square.file();
        score += Tuned::king_openness(file_openness(king_file, our_pawns, their_pawns));
        let bishops = pos.col_piece_bb(color, Bishop);
        for bishop in bishops {
            let (diag, len) = diagonal_openness(bishop, our_pawns, their_pawns);
            score += Tuned::bishop_openness(diag, len);
            let (anti_diag, len) = anti_diagonal_openness(bishop, our_pawns, their_pawns);
            score += Tuned::bishop_openness(anti_diag, len);
        }
        score
    }

    fn checking(pos: &Board, color: Color, generator: &ChessSliderGenerator) -> [Bitboard; 5] {
        let mut result = [Bitboard::default(); 5];
        let square = pos.king_sq(color);
        result[Pawn as usize] = Board::single_pawn_captures(!color, square);
        result[Knight as usize] = Board::knight_attacks_from(square);
        result[Bishop as usize] = generator.bishop_attacks(square);
        result[Rook as usize] = generator.rook_attacks(square);
        result[Queen as usize] = result[Rook as usize] | result[Bishop as usize];
        result
    }

    fn pins_and_discovered_checks(state: &mut EvalState<Tuned>, pos: &Board, color: Color) -> Tuned::Score {
        let mut score = Tuned::Score::default();
        let their_king = pos.king_sq(!color);
        let blockers = pos.occupied_bb();
        let rook_sliders = (pos.piece_bb(Rook) | pos.piece_bb(Queen)) & pos.player_bb(color);
        for slider in rook_sliders {
            let ray = Bitboard::ray_exclusive(slider, their_king, ChessboardSize::default());
            let blockers = ray & blockers;
            if blockers.is_single_piece() && (slider.rank() == their_king.rank() || slider.file() == their_king.file())
            {
                let piece = pos.piece_type_on(blockers.ones().next().unwrap());
                if blockers.intersects(pos.player_bb(color)) {
                    score += Tuned::discovered_check(piece);
                    if piece != Pawn {
                        state.stm_bonus[color] += Tuned::discovered_check_stm();
                    }
                } else {
                    score += Tuned::pin(piece)
                }
            }
        }
        let bishop_sliders = (pos.piece_bb(Bishop) | pos.piece_bb(Queen)) & pos.player_bb(color);
        for slider in bishop_sliders {
            let ray = Bitboard::ray_exclusive(slider, their_king, ChessboardSize::default());
            let blockers = ray & blockers;
            if blockers.is_single_piece() && (slider.rank() != their_king.rank() && slider.file() != their_king.file())
            {
                let piece = pos.piece_type_on(blockers.ones().next().unwrap());
                if blockers.intersects(pos.player_bb(color)) {
                    score += Tuned::discovered_check(piece);
                    if piece != Pawn {
                        state.stm_bonus[color] += Tuned::discovered_check_stm();
                    }
                } else {
                    score += Tuned::pin(piece)
                }
            }
        }
        score
    }

    fn mobility_and_threats(
        state: &mut EvalState<Tuned>,
        pos: &Board,
        us: Color,
        their_attacks: Bitboard,
    ) -> (Tuned::Score, Bitboard) {
        let mut score = Tuned::Score::default();
        let generator = pos.slider_generator();

        let checking_squares = Self::checking(pos, !us, &generator);

        let attacked_by_pawn = pos.col_piece_bb(!us, Pawn).pawn_attacks(!us);
        let king_zone = Board::normal_king_attacks_from(pos.king_sq(!us));
        let our_pawns = pos.col_piece_bb(us, Pawn);
        // handling double pawn pushes lost elo, somehow
        let pawn_advance_threats = (our_pawns.pawn_advance(us) & pos.empty_bb()).pawn_attacks(us);
        let passer_close = (pos.player_bb(us) & state.passers).moore_inclusive();
        let pawn_attacks = our_pawns.pawn_attacks(us);
        if pawn_attacks.intersects(king_zone) {
            score += Tuned::king_zone_attack(Pawn);
        }
        let mut all_attacks = pawn_attacks;
        // let pawn_king_attacks = (pawn_attacks & king_zone).num_ones();
        // score += Tuned::king_zone_attack(Pawn) * pawn_king_attacks;
        for piece in PieceType::pieces() {
            let protected_by_pawns = pawn_attacks & pos.col_piece_bb(us, piece);
            score += Tuned::pawn_protection(piece) * protected_by_pawns.num_ones();
            let attacked_by_pawns = pawn_attacks & pos.col_piece_bb(!us, piece);
            score += Tuned::pawn_attack(piece) * attacked_by_pawns.num_ones();
            let threatened_by_pawn_advance = pawn_advance_threats & pos.col_piece_bb(!us, piece);
            score += Tuned::pawn_advance_threat(piece) * threatened_by_pawn_advance.num_ones();
        }
        for piece in PieceType::non_pawn_pieces() {
            for square in pos.col_piece_bb(us, piece) {
                // TODO: Maybe it makes sense to ensure the compiler unrolls this loop
                let attacks = Board::threatening_attacks(square, piece, us, &generator);
                all_attacks |= attacks;
                let attacks_no_pawn_recapture = attacks & !attacked_by_pawn;
                let attacks_no_recapture = attacks & !their_attacks;
                let mobility = (attacks_no_pawn_recapture & !pos.player_bb(us)).num_ones();
                score += Tuned::mobility(piece, mobility);
                let safe_squares = (attacks_no_recapture & !pos.player_bb(us)).num_ones().min(MAX_SAFE_MOBILITY);
                score += Tuned::safe_squares(piece, safe_squares);
                for threatened_piece in PieceType::pieces() {
                    let attacked = pos.col_piece_bb(!us, threatened_piece) & attacks;
                    score += Tuned::threats(piece, threatened_piece) * attacked.num_ones();
                    let defended = pos.col_piece_bb(us, threatened_piece) & attacks_no_pawn_recapture;
                    score += Tuned::defended(piece, threatened_piece) * defended.num_ones();
                }
                if attacks_no_pawn_recapture.intersects(king_zone) {
                    score += Tuned::king_zone_attack(piece);
                }
                if piece != King {
                    if attacks_no_pawn_recapture.intersects(checking_squares[piece as usize] & !pos.player_bb(us)) {
                        score += Tuned::can_give_check(piece);
                        state.stm_bonus[us] += Tuned::check_stm();
                    }
                    if attacks_no_recapture.intersects(checking_squares[piece as usize] & !pos.player_bb(us)) {
                        score += Tuned::safe_check(piece);
                        state.stm_bonus[us] += Tuned::safe_check_stm();
                    }
                }

                if attacks.intersects(passer_close) {
                    score += Tuned::passer_protection();
                }
            }
        }
        (score, all_attacks)
    }

    // should be called last because it uses information set by other functions
    fn recomputed_every_time(state: &mut EvalState<Tuned>, pos: &Board) -> Tuned::Score {
        let mut score = Tuned::Score::default();
        state.stm_bonus = [Tuned::Score::default(), Tuned::Score::default()];
        let us = pos.active_player();
        let mut their_attacks = pos.threats();
        for color in [us, !us] {
            score += Self::bishop_pair(pos, color);
            score += Self::bad_bishop(pos, color);
            score += Self::open_lines(pos, color);
            let (s, a) = Self::mobility_and_threats(state, pos, color, their_attacks);
            score += s;
            their_attacks = a;
            score += Self::pins_and_discovered_checks(state, pos, color);
            score = -score;
        }
        let score = if us.is_first() { score } else { -score };
        score + Self::useless_material_advantage(state, pos)
    }

    fn psqt_delta(
        &self,
        old_pos: &Board,
        mov: Move,
        captured: PieceType,
        new_pos: &Board,
    ) -> (Tuned::Score, PhaseType) {
        let moving_player = old_pos.active_player();
        let mut delta = Tuned::Score::default();
        let mut phase_delta = PhaseType::default();
        let piece = mov.piece_type(old_pos);
        let mirror = new_pos.king_sq(moving_player).file() < 4;
        let src_sq = mov.src_square().flip_horizontal_if(mirror);
        let dest_sq = mov.dest_square().flip_horizontal_if(mirror);
        debug_assert!(!(piece == King && (mov.src_square().file() < 4) != (new_pos.king_sq(moving_player).file() < 4)));
        delta -= self.tuned.psqt(src_sq, piece, moving_player);
        if mov.is_castle() {
            let side = mov.castle_side();
            debug_assert_eq!(side, Kingside); // otherwise, we would have mirrored the psqts.
            delta += self.tuned.psqt(new_pos.king_sq(moving_player), King, moving_player);
            // since PSQTs are player-relative, castling always takes place on the 0th rank
            let rook_dest_square = Square::from_rank_file(7, side.rook_dest_file());
            let rook_start_square = Square::from_rank_file(7, old_pos.rook_start_file(moving_player, side));
            delta += self.tuned.psqt(rook_dest_square, Rook, Black);
            delta -= self.tuned.psqt(rook_start_square, Rook, Black);
        } else if mov.is_promotion() {
            delta += self.tuned.psqt(dest_sq, mov.promo_piece(), moving_player);
            phase_delta += CHESS_PIECE_PHASE[mov.promo_piece() as usize];
        } else {
            delta += self.tuned.psqt(dest_sq, piece, moving_player);
        }
        let mirror_other = new_pos.king_sq(!moving_player).file() < 4;
        if let Some(ep_sq) = mov.square_of_pawn_taken_by_ep() {
            delta += self.tuned.psqt(ep_sq.flip_horizontal_if(mirror_other), Pawn, !moving_player);
        } else if captured != Empty {
            // capturing a piece increases our score by the piece's psqt value from the opponent's point of view
            delta +=
                self.tuned.psqt(mov.dest_square().flip_horizontal_if(mirror_other), captured, moving_player.other());
            phase_delta -= CHESS_PIECE_PHASE[captured as usize];
        }
        // the position is always evaluated from white's perspective
        (
            match moving_player {
                White => delta,
                Black => -delta,
            },
            phase_delta,
        )
    }

    fn eval_from_scratch(&self, pos: &Board) -> EvalState<Tuned> {
        let (material, psqt_score) = self.psqt(pos);
        let mut state = EvalState {
            hash: pos.hash_pos(),
            pawn_key: pos.pawn_key(),
            passers: Default::default(),
            phase: pos.phase(),
            material,
            psqt_score,
            pawn_score: Default::default(),
            stm_bonus: Default::default(),
            total_score: Default::default(),
        };
        state.pawn_score = Self::pawns(pos, &mut state.passers);
        state.total_score = Self::recomputed_every_time(&mut state, pos)
            + state.material.clone()
            + state.psqt_score.clone()
            + state.pawn_score.clone();
        state
    }

    pub fn do_eval(&self, pos: &Board) -> <Tuned::Score as ScoreType>::Finalized {
        let state = self.eval_from_scratch(pos);
        state.total_score.finalize(
            state.phase,
            24,
            pos.active_player(),
            <Tuned::Score as ScoreType>::Finalized::default(),
            &state.stm_bonus,
        )
    }

    fn incremental(&self, mut state: EvalState<Tuned>, old_pos: &Board, mov: Move, new_pos: &Board) -> EvalState<Tuned>
    where
        Tuned::Score: Display,
    {
        // test both the total hash and the pawn key as a band-aid mitigation against hash collisions
        // (which should basically never happen in a game, but could be set up manually using `position` commands)
        if old_pos.hash_pos() != state.hash || old_pos.pawn_key() != state.pawn_key {
            return self.eval_from_scratch(new_pos);
        } else if mov == Move::default() {
            debug_assert_eq!(&old_pos.make_nullmove().unwrap(), new_pos);
            return state;
        }
        debug_assert_eq!(
            self.psqt(old_pos),
            (state.material.clone(), state.psqt_score.clone()),
            "{0:?} {1:?} {old_pos} {new_pos} {2}",
            self.psqt(old_pos),
            state.psqt_score,
            mov.compact_formatter(old_pos)
        );
        debug_assert_eq!(&old_pos.make_move(mov).unwrap(), new_pos);

        let piece_type = mov.piece_type(old_pos);
        let captured = mov.captured(old_pos);

        let mut material_delta = Tuned::Score::default();
        material_delta += Tuned::material(captured);
        if mov.is_promotion() {
            material_delta -= Tuned::material(Pawn);
            material_delta += Tuned::material(mov.promo_piece());
        }
        if !old_pos.active_player().is_first() {
            material_delta = -material_delta;
        }
        state.material += material_delta;
        // also deals with castles, unlike testing mov.dest_square().rank()
        if piece_type == King && (mov.src_square().file() < 4) != (new_pos.king_sq(old_pos.active_player()).file() < 4)
        {
            state.psqt_score = self.psqt(new_pos).1;
            state.phase = 0;
            for piece in PieceType::non_king_pieces() {
                state.phase += new_pos.piece_bb(piece).num_ones() as isize * CHESS_PIECE_PHASE[piece as usize];
            }
        } else {
            let (psqt_delta, phase_delta) = self.psqt_delta(old_pos, mov, captured, new_pos);
            state.psqt_score += psqt_delta;
            state.phase += phase_delta;
            debug_assert_eq!(
                (state.material.clone(), state.psqt_score.clone()),
                self.psqt(new_pos),
                "({0}, {1}) ({2}, {3}) {4} {old_pos} {new_pos} {5}",
                state.material.clone(),
                state.psqt_score.clone(),
                self.psqt(new_pos).0,
                self.psqt(new_pos).1,
                self.psqt_delta(old_pos, mov, captured, new_pos).0,
                mov.compact_formatter(old_pos)
            );
        }
        let in_front_of_pawns = old_pos.col_piece_bb(White, Pawn).pawn_advance(White)
            | old_pos.col_piece_bb(Black, Pawn).pawn_advance(Black);
        let maybe_pawn_eval_change =
            in_front_of_pawns.has(mov.src_square()) || in_front_of_pawns.has(mov.dest_square());
        if matches!(piece_type, Pawn | King) || captured == Pawn || maybe_pawn_eval_change {
            state.pawn_score = Self::pawns(new_pos, &mut state.passers);
        }
        state.hash = new_pos.hash_pos();
        state.pawn_key = new_pos.pawn_key();
        state.total_score = Self::recomputed_every_time(&mut state, new_pos)
            + state.material.clone()
            + state.psqt_score.clone()
            + state.pawn_score.clone();
        state
    }
}

fn eval_lite<Tuned: LiteValues<Score = PhasedScore>>(
    this: &mut GenericLiTEval<Tuned>,
    pos: &Board,
    ply: usize,
) -> Score {
    let state = this.eval_from_scratch(pos);
    this.stack[ply] = state;
    state.total_score.finalize(state.phase, 24, pos.active_player(), TEMPO, &state.stm_bonus)
}

fn eval_lite_incremental<Tuned: LiteValues<Score = PhasedScore>>(
    this: &mut GenericLiTEval<Tuned>,
    old_pos: &Board,
    mov: Move,
    new_pos: &Board,
    ply: usize,
) -> Score {
    debug_assert!(ply > 0);
    let prev = this.stack[ply - 1];
    if this.stack[ply].hash != new_pos.hash_pos() {
        this.stack[ply] = this.incremental(prev, old_pos, mov, new_pos);
    }
    this.stack[ply].total_score.finalize(
        this.stack[ply].phase,
        24,
        new_pos.active_player(),
        TEMPO,
        &this.stack[ply].stm_bonus,
    )
}

impl Eval<Board> for LiTEval {
    fn eval(&mut self, pos: &Board, ply: usize, _engine: Color) -> Score {
        eval_lite(self, pos, ply)
    }

    // Zobrist hash collisions should be rare enough not to matter, and even when they occur,
    // they won't cause a crash except for failing a debug assertion, which isn't enabled in release mode
    fn eval_incremental(&mut self, old_pos: &Board, mov: Move, new_pos: &Board, ply: usize, _engine: Color) -> Score {
        eval_lite_incremental(self, old_pos, mov, new_pos, ply)
    }

    fn piece_scale(&self) -> ScoreT {
        5
    }
}

impl Eval<Board> for KingGambot {
    fn eval(&mut self, pos: &Board, ply: usize, engine: Color) -> Score {
        self.tuned.us = engine;
        eval_lite(self, pos, ply)
    }

    fn eval_incremental(&mut self, old_pos: &Board, mov: Move, new_pos: &Board, ply: usize, engine: Color) -> Score {
        if engine != self.tuned.us {
            self.eval(new_pos, ply, engine)
        } else {
            eval_lite_incremental(self, old_pos, mov, new_pos, ply)
        }
    }

    fn piece_scale(&self) -> ScoreT {
        5
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gears::games::chess::Board;
    use gears::general::board::BoardHelpers;
    use gears::general::board::Strictness::Strict;

    #[test]
    fn test_symmetry() {
        let pos = Board::default();
        let mut eval = LiTEval::default();
        let e = eval.eval(&pos, 0, White);
        assert_eq!(e, TEMPO);
        assert_eq!(e, eval.eval(&pos, 0, Black));
        assert_eq!(e, eval.eval(&pos, 1, Black));
        let pos = Board::from_fen("1k6/p6r/4p3/8/8/4P3/P6R/1K6 w - - 0 1", Strict).unwrap();
        let e = eval.eval(&pos, 0, White);
        assert!(e >= TEMPO);
        assert!(e <= Score(300));
        let pos = Board::from_fen("1k6/p6n/4p3/8/8/4P3/P6N/1K6 w - - 0 1", Strict).unwrap();
        let e = eval.eval(&pos, 0, White);
        assert_eq!(e, TEMPO);
        let e2 = eval.eval(&pos.make_nullmove().unwrap(), 0, Black);
        assert_eq!(e - TEMPO, -e2 + TEMPO);
    }

    #[test]
    fn test_mirroring() {
        let mut eval = LiTEval::default();
        let pos = Board::from_fen("4k3/p6p/8/8/8/8/P6P/4K3 w - - 0 1", Strict).unwrap();
        let new_pos = pos.make_move_from_str("Kd1").unwrap().make_move_from_str("Kd8").unwrap();
        assert_eq!(eval.eval(&pos, 0, White), eval.eval(&new_pos, 0, White));
        let pos1 = Board::from_fen("4k3/8/1q4pn/8/1B6/1R6/P7/4K3 w - - 0 1", Strict).unwrap();
        let pos2 = Board::from_fen("3k4/8/np4q1/8/6B1/6R1/7P/3K4 w - - 0 1", Strict).unwrap();
        assert_eq!(eval.eval(&pos1, 0, White), eval.eval(&pos2, 0, White));
    }
}
