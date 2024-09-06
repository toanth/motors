use std::fmt::Display;
use strum::IntoEnumIterator;

use crate::eval::chess::lite_values::*;
use crate::eval::chess::{pawn_shield_idx, DiagonalOpenness, FileOpenness};
use gears::games::chess::moves::ChessMove;
use gears::games::chess::pieces::ChessPieceType::*;
use gears::games::chess::pieces::{ChessPieceType, NUM_CHESS_PIECES};
use gears::games::chess::squares::ChessSquare;
use gears::games::chess::ChessColor::{Black, White};
use gears::games::chess::{ChessColor, Chessboard};
use gears::games::Color;
use gears::games::{DimT, ZobristHash};
use gears::general::bitboards::chess::{
    ChessBitboard, A_FILE, CHESS_ANTI_DIAGONALS, CHESS_DIAGONALS,
};
use gears::general::bitboards::Bitboard;
use gears::general::bitboards::RawBitboard;
use gears::general::board::Board;
use gears::general::common::StaticallyNamedEntity;
use gears::general::moves::Move;
use gears::general::squares::RectangularCoordinates;
use gears::score::{PhaseType, Score, ScoreT};

use crate::eval::chess::lite::FileOpenness::{Closed, Open, SemiClosed, SemiOpen};
use crate::eval::{Eval, ScoreType};

#[derive(Debug, Default, Copy, Clone)]
struct EvalState<Tuned: LiteValues> {
    hash: ZobristHash,
    phase: PhaseType,
    // scores are stored from the perspective of the white player
    psqt_score: Tuned::Score,
}

#[derive(Default, Debug, Clone)]
pub struct GenericLiTEval<Tuned: LiteValues> {
    stack: Vec<EvalState<Tuned>>,
}

pub type LiTEval = GenericLiTEval<Lite>;

pub const TEMPO: Score = Score(10);
// TODO: Differentiate between rooks and kings in front of / behind pawns?

/// Includes a phase for the empty piece to simplify the implementation
const PIECE_PHASE: [PhaseType; NUM_CHESS_PIECES + 1] = [0, 1, 1, 2, 4, 0, 0];

fn openness(
    ray: ChessBitboard,
    our_pawns: ChessBitboard,
    their_pawns: ChessBitboard,
) -> FileOpenness {
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

pub fn file_openness(
    file: DimT,
    our_pawns: ChessBitboard,
    their_pawns: ChessBitboard,
) -> FileOpenness {
    let file = ChessBitboard::file_no(file);
    openness(file, our_pawns, their_pawns)
}

pub fn diagonal_openness(
    square: ChessSquare,
    our_pawns: ChessBitboard,
    their_pawns: ChessBitboard,
) -> (DiagonalOpenness, usize) {
    let diag = CHESS_DIAGONALS[square.bb_idx()];
    (openness(diag, our_pawns, their_pawns), diag.num_ones())
}

pub fn anti_diagonal_openness(
    square: ChessSquare,
    our_pawns: ChessBitboard,
    their_pawns: ChessBitboard,
) -> (DiagonalOpenness, usize) {
    let anti_diag = CHESS_ANTI_DIAGONALS[square.bb_idx()];
    (
        openness(anti_diag, our_pawns, their_pawns),
        anti_diag.num_ones(),
    )
}

impl<Tuned: LiteValues> StaticallyNamedEntity for GenericLiTEval<Tuned> {
    fn static_short_name() -> impl Display
    where
        Self: Sized,
    {
        "LiTE"
    }

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        "Chess LiTE: Linear Tuned Eval for Chess".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "A classical evaluation for chess, tuned using 'pliers'".to_string()
    }
}

impl<Tuned: LiteValues> GenericLiTEval<Tuned> {
    fn psqt(pos: &Chessboard) -> Tuned::Score {
        let mut res = Tuned::Score::default();
        for color in ChessColor::iter() {
            for piece in ChessPieceType::pieces() {
                for square in pos.colored_piece_bb(color, piece).ones() {
                    res += Tuned::psqt(square, piece, color);
                }
            }
            res = -res;
        }
        res
    }

    fn bishop_pair(
        pos: &Chessboard,
        color: ChessColor,
    ) -> <Tuned::Score as ScoreType>::SingleFeatureScore {
        if pos.colored_piece_bb(color, Bishop).more_than_one_bit_set() {
            Tuned::bishop_pair()
        } else {
            Default::default()
        }
    }

    fn pawn_shield(
        pos: &Chessboard,
        color: ChessColor,
    ) -> <Tuned::Score as ScoreType>::SingleFeatureScore {
        let our_pawns = pos.colored_piece_bb(color, Pawn);
        let king_square = pos.king_square(color);
        let idx = pawn_shield_idx(our_pawns, king_square, color);
        Tuned::pawn_shield(idx)
    }

    fn pawns(pos: &Chessboard, color: ChessColor) -> Tuned::Score {
        let our_pawns = pos.colored_piece_bb(color, Pawn);
        let their_pawns = pos.colored_piece_bb(color.other(), Pawn);
        let mut score = Tuned::Score::default();

        for square in our_pawns.ones() {
            let normalized_square = square.flip_if(color == White);
            let in_front =
                (A_FILE << (square.flip_if(color == Black).bb_idx() + 8)).flip_if(color == Black);
            let blocking = in_front | in_front.west() | in_front.east();
            if (in_front & our_pawns).is_zero() && (blocking & their_pawns).is_zero() {
                score += Tuned::passed_pawn(normalized_square);
            }
            let file = ChessBitboard::file_no(square.file());
            let neighbors = file.west() | file.east();
            let supporting = neighbors & !blocking;
            if (supporting & our_pawns).is_zero() {
                score += Tuned::unsupported_pawn();
            }
        }
        let num_doubled_pawns = (our_pawns & (our_pawns.north())).num_ones();
        score += Tuned::doubled_pawn() * num_doubled_pawns;
        for piece in ChessPieceType::pieces() {
            let bb = pos.colored_piece_bb(color, piece);
            let pawn_attacks = our_pawns.pawn_attacks(color);
            let protected_by_pawns = pawn_attacks & bb;
            score += Tuned::pawn_protection(piece) * protected_by_pawns.num_ones();
            let attacked_by_pawns = pawn_attacks & pos.colored_piece_bb(color.other(), piece);
            score += Tuned::pawn_attack(piece) * attacked_by_pawns.num_ones();
        }

        score
    }

    fn open_lines(pos: &Chessboard, color: ChessColor) -> Tuned::Score {
        let mut score = Tuned::Score::default();
        let our_pawns = pos.colored_piece_bb(color, Pawn);
        let their_pawns = pos.colored_piece_bb(color.other(), Pawn);
        // Rooks on (semi)open/closed files (semi-closed files are handled by adjusting the base rook values during tuning)
        let rooks = pos.colored_piece_bb(color, Rook);
        for rook in rooks.ones() {
            score += Tuned::rook_openness(file_openness(rook.file(), our_pawns, their_pawns));
        }
        // King on (semi)open/closed file
        let king_square = pos.king_square(color);
        let king_file = king_square.file();
        score += Tuned::king_openness(file_openness(king_file, our_pawns, their_pawns));
        let bishops = pos.colored_piece_bb(color, Bishop);
        for bishop in bishops.ones() {
            let (diag, len) = diagonal_openness(bishop, our_pawns, their_pawns);
            score += Tuned::bishop_openness(diag, len);
            let (anti_diag, len) = anti_diagonal_openness(bishop, our_pawns, their_pawns);
            score += Tuned::bishop_openness(anti_diag, len);
        }
        score
    }

    fn mobility_and_threats(pos: &Chessboard, color: ChessColor) -> Tuned::Score {
        let mut score = Tuned::Score::default();
        let attacked_by_pawn = pos
            .colored_piece_bb(color.other(), Pawn)
            .pawn_attacks(color.other());
        let king_zone = Chessboard::normal_king_moves_from_square(pos.king_square(color.other()));
        if (pos.colored_piece_bb(color, Pawn).pawn_attacks(color) & king_zone).has_set_bit() {
            score += Tuned::king_zone_attack(Pawn);
        }
        for piece in ChessPieceType::non_pawn_pieces() {
            for square in pos.colored_piece_bb(color, piece).ones() {
                let attacks =
                    pos.attacks_no_castle_or_pawn_push(square, piece, color) & !attacked_by_pawn;
                let mobility = (attacks & !pos.colored_bb(color)).num_ones();
                score += Tuned::mobility(piece, mobility);
                for threatened_piece in ChessPieceType::pieces() {
                    let attacked = pos.colored_piece_bb(color.other(), threatened_piece) & attacks;
                    score += Tuned::threats(piece, threatened_piece) * attacked.num_ones();
                    let defended = pos.colored_piece_bb(color, threatened_piece) & attacks;
                    score += Tuned::defended(piece, threatened_piece) * defended.num_ones();
                }
                if (attacks & king_zone).has_set_bit() {
                    score += Tuned::king_zone_attack(piece);
                }
            }
        }
        score
    }

    fn recomputed_every_time(pos: &Chessboard) -> Tuned::Score {
        let mut score = Tuned::Score::default();
        for color in ChessColor::iter() {
            score += Self::bishop_pair(pos, color);
            score += Self::pawns(pos, color);
            score += Self::pawn_shield(pos, color);
            score += Self::open_lines(pos, color);
            score += Self::mobility_and_threats(pos, color);
            score = -score;
        }
        score
    }

    fn psqt_delta(
        old_pos: &Chessboard,
        mov: ChessMove,
        new_pos: &Chessboard,
    ) -> (Tuned::Score, PhaseType) {
        let moving_player = old_pos.active_player();
        // the current player has been flipped
        let mut delta = Tuned::Score::default();
        let mut phase_delta = PhaseType::default();
        let piece = mov.piece_type();
        let captured = mov.captured(old_pos);
        delta -= Tuned::psqt(mov.src_square(), piece, moving_player);
        if mov.is_castle() {
            let side = mov.castle_side();
            delta += Tuned::psqt(new_pos.king_square(moving_player), King, moving_player);
            // since PSQTs are player-relative, castling always takes place on the 0th rank
            let rook_dest_square = ChessSquare::from_rank_file(7, side.rook_dest_file());
            let rook_start_square =
                ChessSquare::from_rank_file(7, old_pos.rook_start_file(moving_player, side));
            delta += Tuned::psqt(rook_dest_square, Rook, Black);
            delta -= Tuned::psqt(rook_start_square, Rook, Black);
        } else if mov.promo_piece() == Empty {
            delta += Tuned::psqt(mov.dest_square(), piece, moving_player);
        } else {
            delta += Tuned::psqt(mov.dest_square(), mov.promo_piece(), moving_player);
            phase_delta += PIECE_PHASE[mov.promo_piece() as usize];
        }
        if mov.is_ep() {
            delta += Tuned::psqt(
                mov.square_of_pawn_taken_by_ep().unwrap(),
                Pawn,
                moving_player.other(),
            );
        } else if captured != Empty {
            // capturing a piece increases our score by the piece's psqt value from the opponent's point of view
            delta += Tuned::psqt(mov.dest_square(), captured, moving_player.other());
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

    fn eval_from_scratch(pos: &Chessboard) -> (EvalState<Tuned>, Tuned::Score) {
        let mut state = EvalState::default();

        let mut phase = 0;
        for piece in ChessPieceType::non_king_pieces() {
            phase += pos.piece_bb(piece).num_ones() as isize * PIECE_PHASE[piece as usize];
        }
        state.phase = phase;

        let psqt_score = Self::psqt(pos);
        state.psqt_score = psqt_score.clone();
        state.hash = pos.zobrist_hash();
        let score: Tuned::Score = Self::recomputed_every_time(pos) + psqt_score;
        (state, score)
    }

    pub fn do_eval(pos: &Chessboard) -> <Tuned::Score as ScoreType>::Finalized {
        let (state, score) = Self::eval_from_scratch(pos);
        score.finalize(
            state.phase,
            24,
            pos.active_player(),
            <Tuned::Score as ScoreType>::Finalized::default(),
        )
    }

    fn incremental(
        mut state: EvalState<Tuned>,
        old_pos: &Chessboard,
        mov: ChessMove,
        new_pos: &Chessboard,
    ) -> (EvalState<Tuned>, Tuned::Score)
    where
        Tuned::Score: Display,
    {
        if old_pos.zobrist_hash() != state.hash {
            return Self::eval_from_scratch(new_pos);
        }
        if mov != ChessMove::default() {
            // null moves are encoded as a1a1, but it's possible that there's a "captured" piece on a1
            debug_assert_eq!(
                Self::psqt(old_pos),
                state.psqt_score,
                "{0} {1} {old_pos} {new_pos} {mov}",
                Self::psqt(old_pos),
                state.psqt_score
            );
            debug_assert_eq!(&old_pos.make_move(mov).unwrap(), new_pos);
            let (psqt_delta, phase_delta) = Self::psqt_delta(old_pos, mov, new_pos);
            state.psqt_score += psqt_delta;
            state.phase += phase_delta;
            debug_assert_eq!(
                state.psqt_score,
                Self::psqt(new_pos),
                "{0} {1} {2} {old_pos} {new_pos} {mov}",
                state.psqt_score,
                Self::psqt(new_pos),
                Self::psqt_delta(old_pos, mov, new_pos).0,
            );
        }
        state.hash = new_pos.zobrist_hash();
        let score = Self::recomputed_every_time(new_pos) + state.psqt_score.clone();
        (state, score)
    }
}

impl Eval<Chessboard> for LiTEval {
    fn eval(&mut self, pos: &Chessboard) -> Score {
        self.stack.clear();
        let (state, score) = Self::eval_from_scratch(pos);
        self.stack.push(state);
        let score = score.finalize(state.phase, 24, pos.active_player(), TEMPO);
        score   * (100 - pos.halfmove_repetition_clock() as ScoreT) / 100
    }

    // Zobrist hash collisions should be rare enough not to matter, and even when they occur,
    // they won't cause a crash except for failing a debug assertion, which isn't enabled in release mode
    fn eval_incremental(
        &mut self,
        old_pos: &Chessboard,
        mov: ChessMove,
        new_pos: &Chessboard,
        ply: usize,
    ) -> Score {
        debug_assert!(self.stack.len() >= ply);
        debug_assert!(ply > 0);
        let entry = self.stack[ply - 1];
        let (entry, score) = Self::incremental(entry, old_pos, mov, new_pos);
        self.stack.resize(ply + 1, entry);
        score.finalize(entry.phase, 24, new_pos.active_player(), TEMPO)
    }
}
