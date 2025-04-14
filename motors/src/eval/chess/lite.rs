use std::fmt::Display;

use crate::eval::chess::lite_values::*;
use crate::eval::chess::{
    DiagonalOpenness, FLANK, FileOpenness, REACHABLE_PAWNS, pawn_advanced_center_idx, pawn_passive_center_idx,
    pawn_shield_idx,
};
use gears::games::Color;
use gears::games::chess::ChessColor::{Black, White};
use gears::games::chess::moves::ChessMove;
use gears::games::chess::pieces::ChessPieceType::*;
use gears::games::chess::pieces::{ChessPieceType, NUM_CHESS_PIECES};
use gears::games::chess::squares::{ChessSquare, ChessboardSize};
use gears::games::chess::{ChessBitboardTrait, ChessColor, Chessboard};
use gears::games::{DimT, PosHash};
use gears::general::bitboards::RawBitboard;
use gears::general::bitboards::chessboard::{COLORED_SQUARES, ChessBitboard};
use gears::general::bitboards::{Bitboard, KnownSizeBitboard};
use gears::general::board::{BitboardBoard, Board};
use gears::general::common::StaticallyNamedEntity;
use gears::general::hq::ChessSliderGenerator;
use gears::general::moves::Move;
use gears::general::squares::RectangularCoordinates;
use gears::score::{PhaseType, PhasedScore, Score, ScoreT};

use crate::eval::chess::king_gambot::KingGambotValues;
use crate::eval::chess::lite::FileOpenness::{Closed, Open, SemiClosed, SemiOpen};
use crate::eval::{Eval, ScoreType, SingleFeatureScore};

#[derive(Debug, Default, Copy, Clone)]
struct EvalState<Tuned: LiteValues> {
    hash: PosHash,
    pawn_key: PosHash,
    passers: ChessBitboard,
    phase: PhaseType,
    // scores are stored from the perspective of the white player
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

pub const TEMPO: Score = Score(10);
// TODO: Differentiate between rooks and kings in front of / behind pawns?

/// Includes a phase for the empty piece to simplify the implementation
const PIECE_PHASE: [PhaseType; NUM_CHESS_PIECES + 1] = [0, 1, 1, 2, 4, 0, 0];

fn openness(ray: ChessBitboard, our_pawns: ChessBitboard, their_pawns: ChessBitboard) -> FileOpenness {
    if (ray & our_pawns).is_zero() && (ray & their_pawns).is_zero() {
        Open
    } else if (ray & our_pawns).is_zero() {
        SemiOpen
    } else if (ray & our_pawns).has_set_bit() && (ray & their_pawns).has_set_bit() {
        Closed
    } else {
        SemiClosed
    }
}

pub fn file_openness(file: DimT, our_pawns: ChessBitboard, their_pawns: ChessBitboard) -> FileOpenness {
    let file = ChessBitboard::file(file);
    openness(file, our_pawns, their_pawns)
}

pub fn diagonal_openness(
    square: ChessSquare,
    our_pawns: ChessBitboard,
    their_pawns: ChessBitboard,
) -> (DiagonalOpenness, usize) {
    // TODO: don't pass size
    let diag = ChessBitboard::diag_for_sq(square, ChessboardSize::default());
    (openness(diag, our_pawns, their_pawns), diag.num_ones())
}

pub fn anti_diagonal_openness(
    square: ChessSquare,
    our_pawns: ChessBitboard,
    their_pawns: ChessBitboard,
) -> (DiagonalOpenness, usize) {
    let anti_diag = ChessBitboard::anti_diag_for_sq(square, ChessboardSize::default());
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
    fn psqt(&self, pos: &Chessboard) -> Tuned::Score {
        let mut res = Tuned::Score::default();
        for color in ChessColor::iter() {
            for piece in ChessPieceType::pieces() {
                for square in pos.col_piece_bb(color, piece).ones() {
                    res += self.tuned.psqt(square, piece, color);
                }
            }
            res = -res;
        }
        res
    }

    fn bishop_pair(pos: &Chessboard, color: ChessColor) -> SingleFeatureScore<Tuned::Score> {
        if pos.col_piece_bb(color, Bishop).more_than_one_bit_set() { Tuned::bishop_pair() } else { Default::default() }
    }

    fn bad_bishop(pos: &Chessboard, color: ChessColor) -> Tuned::Score {
        let mut score = Tuned::Score::default();
        let pawns = pos.col_piece_bb(color, Pawn);
        for bishop in pos.col_piece_bb(color, Bishop).ones() {
            let sq_color = bishop.square_color();
            score += Tuned::bad_bishop((COLORED_SQUARES[sq_color as usize] & pawns).num_ones());
        }
        score
    }

    fn pawn_shield_for(pos: &Chessboard, color: ChessColor) -> SingleFeatureScore<Tuned::Score> {
        let our_pawns = pos.col_piece_bb(color, Pawn);
        let king_square = pos.king_square(color);
        let idx = pawn_shield_idx(our_pawns, king_square, color);
        Tuned::default().pawn_shield(color, idx)
    }

    fn pawn_center(pos: &Chessboard) -> Tuned::Score {
        let mut score = Tuned::Score::default();
        for color in ChessColor::iter() {
            let advanced_idx = pawn_advanced_center_idx(pos.col_piece_bb(color, Pawn), color);
            let passive_idx = pawn_passive_center_idx(pos.col_piece_bb(color, Pawn), color);
            score += Tuned::pawn_advanced_center(advanced_idx);
            score += Tuned::pawn_passive_center(passive_idx);
            score = -score;
        }
        score
    }

    fn pawns_for(pos: &Chessboard, us: ChessColor, passers: &mut ChessBitboard) -> Tuned::Score {
        let our_pawns = pos.col_piece_bb(us, Pawn);
        let their_pawns = pos.col_piece_bb(us.other(), Pawn);
        let all_pawns = pos.piece_bb(Pawn);
        let mut score = Tuned::Score::default();
        score += Self::pawn_shield_for(pos, us);
        let our_king = pos.king_square(us);
        if (all_pawns & FLANK[our_king.file() as usize]).is_zero() {
            score += Tuned::pawnless_flank();
        }
        for square in our_pawns.ones() {
            let normalized_square = square.flip_if(us == Black);
            let in_front = (ChessBitboard::A_FILE << (square.flip_if(us == Black).bb_idx() + 8)).flip_if(us == Black);
            let blocking_squares = in_front | in_front.west() | in_front.east();
            let file = ChessBitboard::file(square.file());
            let neighbor_files = file.west() | file.east();
            let supporting = neighbor_files & !blocking_squares;
            if (supporting & our_pawns).is_zero() {
                score += Tuned::unsupported_pawn();
            }
            // passed pawn
            if (in_front & our_pawns).is_zero() && (blocking_squares & their_pawns).is_zero() {
                score += Tuned::passed_pawn(normalized_square);
                let their_king = pos.king_square(!us).flip_if(us == Black);
                if REACHABLE_PAWNS[their_king.bb_idx()].is_bit_set(normalized_square) {
                    score += Tuned::stoppable_passer();
                }
                let near_king =
                    Chessboard::normal_king_attacks_from(square) & Chessboard::normal_king_attacks_from(our_king);
                if near_king.has_set_bit() {
                    score += Tuned::close_king_passer();
                }
                if pos.player_bb(!us).is_bit_set(square.pawn_advance_unchecked(us)) {
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
            if (our_pawns & (sq_bb.east() | sq_bb.west())).has_set_bit() {
                score += Tuned::phalanx(normalized_square.rank() - 1);
            }
        }
        let num_doubled_pawns = (our_pawns & (our_pawns.north())).num_ones();
        score += Tuned::doubled_pawn() * num_doubled_pawns;
        score
    }

    fn pawns(pos: &Chessboard, passers: &mut ChessBitboard) -> Tuned::Score {
        *passers = ChessBitboard::default();
        Self::pawn_center(pos) + Self::pawns_for(pos, White, passers) - Self::pawns_for(pos, Black, passers)
    }

    fn open_lines(pos: &Chessboard, color: ChessColor) -> Tuned::Score {
        let mut score = Tuned::Score::default();
        let our_pawns = pos.col_piece_bb(color, Pawn);
        let their_pawns = pos.col_piece_bb(color.other(), Pawn);
        // Rooks on (semi)open/closed files (semi-closed files are handled by adjusting the base rook values during tuning)
        let rooks = pos.col_piece_bb(color, Rook);
        for rook in rooks.ones() {
            score += Tuned::rook_openness(file_openness(rook.file(), our_pawns, their_pawns));
        }
        // King on (semi)open/closed file
        let king_square = pos.king_square(color);
        let king_file = king_square.file();
        score += Tuned::king_openness(file_openness(king_file, our_pawns, their_pawns));
        let bishops = pos.col_piece_bb(color, Bishop);
        for bishop in bishops.ones() {
            let (diag, len) = diagonal_openness(bishop, our_pawns, their_pawns);
            score += Tuned::bishop_openness(diag, len);
            let (anti_diag, len) = anti_diagonal_openness(bishop, our_pawns, their_pawns);
            score += Tuned::bishop_openness(anti_diag, len);
        }
        score
    }

    fn checking(pos: &Chessboard, color: ChessColor, generator: &ChessSliderGenerator) -> [ChessBitboard; 5] {
        let mut result = [ChessBitboard::default(); 5];
        let square = pos.king_square(color);
        result[Pawn as usize] = Chessboard::single_pawn_captures(!color, square);
        result[Knight as usize] = Chessboard::knight_attacks_from(square);
        result[Bishop as usize] = generator.bishop_attacks(square);
        result[Rook as usize] = generator.rook_attacks(square);
        result[Queen as usize] = result[Rook as usize] | result[Bishop as usize];
        result
    }

    fn pins_and_discovered_checks(state: &mut EvalState<Tuned>, pos: &Chessboard, color: ChessColor) -> Tuned::Score {
        let mut score = Tuned::Score::default();
        let their_king = pos.king_square(!color);
        let blockers = pos.occupied_bb();
        let rook_sliders = (pos.piece_bb(Rook) | pos.piece_bb(Queen)) & pos.player_bb(color);
        for slider in rook_sliders.ones() {
            let ray = ChessBitboard::ray_exclusive(slider, their_king, ChessboardSize::default());
            let blockers = ray & blockers;
            if blockers.is_single_piece() && (slider.rank() == their_king.rank() || slider.file() == their_king.file())
            {
                let piece = pos.piece_type_on(blockers.ones().next().unwrap());
                if (blockers & pos.player_bb(color)).has_set_bit() {
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
        for slider in bishop_sliders.ones() {
            let ray = ChessBitboard::ray_exclusive(slider, their_king, ChessboardSize::default());
            let blockers = ray & blockers;
            if blockers.is_single_piece() && (slider.rank() != their_king.rank() && slider.file() != their_king.file())
            {
                let piece = pos.piece_type_on(blockers.ones().next().unwrap());
                if (blockers & pos.player_bb(color)).has_set_bit() {
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

    fn mobility_and_threats(state: &mut EvalState<Tuned>, pos: &Chessboard, us: ChessColor) -> Tuned::Score {
        let mut score = Tuned::Score::default();
        let generator = pos.slider_generator();

        let checking_squares = Self::checking(pos, !us, &generator);

        let attacked_by_pawn = pos.col_piece_bb(us.other(), Pawn).pawn_attacks(us.other());
        let king_zone = Chessboard::normal_king_attacks_from(pos.king_square(us.other()));
        let our_pawns = pos.col_piece_bb(us, Pawn);
        // handling double pawn pushes lost elo, somehow
        let pawn_advance_threats = (our_pawns.pawn_advance(us) & pos.empty_bb()).pawn_attacks(us);
        let passer_close = (pos.player_bb(us) & state.passers).moore_neighbors();
        let pawn_attacks = our_pawns.pawn_attacks(us);
        if (pawn_attacks & king_zone).has_set_bit() {
            score += Tuned::king_zone_attack(Pawn);
        }
        let mut all_attacks = pawn_attacks;
        // let pawn_king_attacks = (pawn_attacks & king_zone).num_ones();
        // score += Tuned::king_zone_attack(Pawn) * pawn_king_attacks;
        for piece in ChessPieceType::pieces() {
            let protected_by_pawns = pawn_attacks & pos.col_piece_bb(us, piece);
            score += Tuned::pawn_protection(piece) * protected_by_pawns.num_ones();
            let attacked_by_pawns = pawn_attacks & pos.col_piece_bb(!us, piece);
            score += Tuned::pawn_attack(piece) * attacked_by_pawns.num_ones();
            let threatened_by_pawn_advance = pawn_advance_threats & pos.col_piece_bb(!us, piece);
            score += Tuned::pawn_advance_threat(piece) * threatened_by_pawn_advance.num_ones();
        }
        for piece in ChessPieceType::non_pawn_pieces() {
            for square in pos.col_piece_bb(us, piece).ones() {
                let attacks = Chessboard::threatening_attacks(square, piece, us, &generator);
                all_attacks |= attacks;
                let attacks_no_pawn_recapture = attacks & !attacked_by_pawn;
                let mobility = (attacks_no_pawn_recapture & !pos.player_bb(us)).num_ones();
                score += Tuned::mobility(piece, mobility);
                for threatened_piece in ChessPieceType::pieces() {
                    let attacked = pos.col_piece_bb(us.other(), threatened_piece) & attacks;
                    score += Tuned::threats(piece, threatened_piece) * attacked.num_ones();
                    if threatened_piece as usize > piece as usize {
                        state.stm_bonus[us] += Tuned::threats_stm() * attacked.num_ones();
                    }
                    let defended = pos.col_piece_bb(us, threatened_piece) & attacks_no_pawn_recapture;
                    score += Tuned::defended(piece, threatened_piece) * defended.num_ones();
                }
                if (attacks_no_pawn_recapture & king_zone).has_set_bit() {
                    score += Tuned::king_zone_attack(piece);
                }
                if piece != King && (attacks_no_pawn_recapture & checking_squares[piece as usize]).has_set_bit() {
                    score += Tuned::can_give_check(piece);
                    state.stm_bonus[us] += Tuned::check_stm();
                }
                if (attacks & passer_close).has_set_bit() {
                    score += Tuned::passer_protection();
                }
            }
        }
        score
    }

    // should be called last because it uses information set by other functions
    fn recomputed_every_time(state: &mut EvalState<Tuned>, pos: &Chessboard) -> Tuned::Score {
        let mut score = Tuned::Score::default();
        state.stm_bonus = [Tuned::Score::default(), Tuned::Score::default()];
        for color in ChessColor::iter() {
            score += Self::bishop_pair(pos, color);
            score += Self::bad_bishop(pos, color);
            score += Self::open_lines(pos, color);
            score += Self::mobility_and_threats(state, pos, color);
            score += Self::pins_and_discovered_checks(state, pos, color);
            score = -score;
        }
        score
    }

    fn psqt_delta(
        &self,
        old_pos: &Chessboard,
        mov: ChessMove,
        captured: ChessPieceType,
        new_pos: &Chessboard,
    ) -> (Tuned::Score, PhaseType) {
        let moving_player = old_pos.active_player();
        // the current player has been flipped
        let mut delta = Tuned::Score::default();
        let mut phase_delta = PhaseType::default();
        let piece = mov.piece_type();
        delta -= self.tuned.psqt(mov.src_square(), piece, moving_player);
        if mov.is_castle() {
            let side = mov.castle_side();
            delta += self.tuned.psqt(new_pos.king_square(moving_player), King, moving_player);
            // since PSQTs are player-relative, castling always takes place on the 0th rank
            let rook_dest_square = ChessSquare::from_rank_file(7, side.rook_dest_file());
            let rook_start_square = ChessSquare::from_rank_file(7, old_pos.rook_start_file(moving_player, side));
            delta += self.tuned.psqt(rook_dest_square, Rook, Black);
            delta -= self.tuned.psqt(rook_start_square, Rook, Black);
        } else if mov.promo_piece() == Empty {
            delta += self.tuned.psqt(mov.dest_square(), piece, moving_player);
        } else {
            delta += self.tuned.psqt(mov.dest_square(), mov.promo_piece(), moving_player);
            phase_delta += PIECE_PHASE[mov.promo_piece() as usize];
        }
        if let Some(ep_sq) = mov.square_of_pawn_taken_by_ep() {
            delta += self.tuned.psqt(ep_sq, Pawn, moving_player.other());
        } else if captured != Empty {
            // capturing a piece increases our score by the piece's psqt value from the opponent's point of view
            delta += self.tuned.psqt(mov.dest_square(), captured, moving_player.other());
            phase_delta -= PIECE_PHASE[captured as usize];
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

    fn eval_from_scratch(&self, pos: &Chessboard) -> EvalState<Tuned> {
        let mut state = EvalState::default();

        let mut phase = 0;
        for piece in ChessPieceType::non_king_pieces() {
            phase += pos.piece_bb(piece).num_ones() as isize * PIECE_PHASE[piece as usize];
        }
        state.phase = phase;

        let psqt_score = self.psqt(pos);
        state.psqt_score = psqt_score.clone();
        let pawn_score = Self::pawns(pos, &mut state.passers);
        state.pawn_score = pawn_score.clone();
        state.hash = pos.hash_pos();
        state.pawn_key = pos.pawn_key();
        state.total_score = Self::recomputed_every_time(&mut state, pos) + psqt_score + pawn_score;
        state
    }

    pub fn do_eval(&self, pos: &Chessboard) -> <Tuned::Score as ScoreType>::Finalized {
        let state = self.eval_from_scratch(pos);
        state.total_score.finalize(
            state.phase,
            24,
            pos.active_player(),
            <Tuned::Score as ScoreType>::Finalized::default(),
            &state.stm_bonus,
        )
    }

    fn incremental(
        &self,
        mut state: EvalState<Tuned>,
        old_pos: &Chessboard,
        mov: ChessMove,
        new_pos: &Chessboard,
    ) -> EvalState<Tuned>
    where
        Tuned::Score: Display,
    {
        // test both the total hash and the pawn key as a band-aid mitigation against hash collisions
        // (which should basically never happen in a game, but could be set up manually using `position` commands)
        if old_pos.hash_pos() != state.hash || old_pos.pawn_key() != state.pawn_key {
            return self.eval_from_scratch(new_pos);
        } else if mov == ChessMove::default() {
            debug_assert_eq!(&old_pos.make_nullmove().unwrap(), new_pos);
            return state;
        }
        debug_assert_eq!(
            self.psqt(old_pos),
            state.psqt_score,
            "{0} {1} {old_pos} {new_pos} {2}",
            self.psqt(old_pos),
            state.psqt_score,
            mov.compact_formatter(old_pos)
        );
        debug_assert_eq!(&old_pos.make_move(mov).unwrap(), new_pos);
        let captured = mov.captured(old_pos);
        let (psqt_delta, phase_delta) = self.psqt_delta(old_pos, mov, captured, new_pos);
        state.psqt_score += psqt_delta;
        state.phase += phase_delta;
        debug_assert_eq!(
            state.psqt_score,
            self.psqt(new_pos),
            "{0} {1} {2} {old_pos} {new_pos} {3}",
            state.psqt_score,
            self.psqt(new_pos),
            self.psqt_delta(old_pos, mov, captured, new_pos).0,
            mov.compact_formatter(old_pos)
        );
        let piece_type = mov.piece_type();
        // TODO: Test if this is actually faster -- getting the captured piece is quite expensive
        // (but this could be remedied by reusing that info from `psqt_delta`, or by using a redundant mailbox)
        // In the long run, move pawn protection / attacks to another function and cache `Self::pawns` as well
        let in_front_of_pawns = old_pos.col_piece_bb(White, Pawn).pawn_advance(White)
            | old_pos.col_piece_bb(Black, Pawn).pawn_advance(Black);
        let maybe_pawn_eval_change =
            in_front_of_pawns.is_bit_set(mov.src_square()) || in_front_of_pawns.is_bit_set(mov.dest_square());
        if matches!(piece_type, Pawn | King) || captured == Pawn || maybe_pawn_eval_change {
            state.pawn_score = Self::pawns(new_pos, &mut state.passers);
        }
        state.hash = new_pos.hash_pos();
        state.pawn_key = new_pos.pawn_key();
        state.total_score =
            Self::recomputed_every_time(&mut state, new_pos) + state.psqt_score.clone() + state.pawn_score.clone();
        state
    }
}

fn eval_lite<Tuned: LiteValues<Score = PhasedScore>>(
    this: &mut GenericLiTEval<Tuned>,
    pos: &Chessboard,
    ply: usize,
) -> Score {
    let state = this.eval_from_scratch(pos);
    this.stack[ply] = state;
    state.total_score.finalize(state.phase, 24, pos.active_player(), TEMPO, &state.stm_bonus)
}

fn eval_lite_incremental<Tuned: LiteValues<Score = PhasedScore>>(
    this: &mut GenericLiTEval<Tuned>,
    old_pos: &Chessboard,
    mov: ChessMove,
    new_pos: &Chessboard,
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

impl Eval<Chessboard> for LiTEval {
    fn eval(&mut self, pos: &Chessboard, ply: usize, _engine: ChessColor) -> Score {
        eval_lite(self, pos, ply)
    }

    // Zobrist hash collisions should be rare enough not to matter, and even when they occur,
    // they won't cause a crash except for failing a debug assertion, which isn't enabled in release mode
    fn eval_incremental(
        &mut self,
        old_pos: &Chessboard,
        mov: ChessMove,
        new_pos: &Chessboard,
        ply: usize,
        _engine: ChessColor,
    ) -> Score {
        eval_lite_incremental(self, old_pos, mov, new_pos, ply)
    }

    fn piece_scale(&self) -> ScoreT {
        5
    }
}

impl Eval<Chessboard> for KingGambot {
    fn eval(&mut self, pos: &Chessboard, ply: usize, engine: ChessColor) -> Score {
        self.tuned.us = engine;
        eval_lite(self, pos, ply)
    }

    fn eval_incremental(
        &mut self,
        old_pos: &Chessboard,
        mov: ChessMove,
        new_pos: &Chessboard,
        ply: usize,
        engine: ChessColor,
    ) -> Score {
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
    use gears::games::chess::Chessboard;
    use gears::general::board::BoardHelpers;
    use gears::general::board::Strictness::Strict;

    #[test]
    fn test_symmetry() {
        let pos = Chessboard::default();
        let mut eval = LiTEval::default();
        let e = eval.eval(&pos, 0, White);
        assert_eq!(e, TEMPO);
        assert_eq!(e, eval.eval(&pos, 0, Black));
        assert_eq!(e, eval.eval(&pos, 1, Black));
        let pos = Chessboard::from_fen("1k6/p6r/4p3/8/8/4P3/P6R/1K6 w - - 0 1", Strict).unwrap();
        let e = eval.eval(&pos, 0, White);
        assert!(e > TEMPO);
        assert!(e <= Score(500));
        let pos = pos.make_move_from_str("Rxh7").unwrap();
        let e = eval.eval(&pos, 0, White);
        assert!(-e > TEMPO + Score(300), "{e}");
        let e2 = eval.eval(&pos.make_nullmove().unwrap(), 0, Black);
        assert!(e - TEMPO > -e2 + TEMPO);
        let pos = Chessboard::from_fen("1k6/p6n/4p3/8/8/4P3/P6N/1K6 w - - 0 1", Strict).unwrap();
        let e = eval.eval(&pos, 0, White);
        let e2 = eval.eval(&pos.make_nullmove().unwrap(), 0, Black);
        assert_eq!(e - TEMPO, -e2 + TEMPO);
    }
}
