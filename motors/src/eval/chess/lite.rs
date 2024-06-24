use strum::IntoEnumIterator;

use crate::eval::chess::lite_values::*;
use crate::eval::chess::{pawn_shield_idx, FileOpenness};
use gears::games::chess::moves::ChessMove;
use gears::games::chess::pieces::UncoloredChessPiece::{Bishop, Empty, King, Pawn, Rook};
use gears::games::chess::pieces::{UncoloredChessPiece, NUM_CHESS_PIECES};
use gears::games::chess::squares::ChessSquare;
use gears::games::chess::Chessboard;
use gears::games::Color::{Black, White};
use gears::games::{Board, Color, DimT, Move, ZobristHash};
use gears::general::bitboards::chess::{ChessBitboard, A_FILE};
use gears::general::bitboards::Bitboard;
use gears::general::bitboards::RawBitboard;
use gears::general::common::StaticallyNamedEntity;
use gears::score::{PhaseType, PhasedScore, Score};

use crate::eval::chess::lite::FileOpenness::{Closed, Open, SemiClosed, SemiOpen};
use crate::eval::Eval;

#[derive(Default, Debug, Clone)]
pub struct LiTEval {
    hash: ZobristHash,
    phase: PhaseType,
    // scores are stored from the perspective of the white player
    psqt_score: PhasedScore,
}

const TEMPO: Score = Score(10);
// TODO: Differentiate between rooks and kings in front of / behind pawns?

/// Includes a phase for the empty piece to simplify the implementation
const PIECE_PHASE: [PhaseType; NUM_CHESS_PIECES + 1] = [0, 1, 1, 2, 4, 0, 0];

pub fn file_openness(
    file: DimT,
    our_pawns: ChessBitboard,
    their_pawns: ChessBitboard,
) -> FileOpenness {
    let file = ChessBitboard::file_no(file);
    if (file & our_pawns).is_zero() && (file & their_pawns).is_zero() {
        Open
    } else if (file & our_pawns).is_zero() {
        SemiOpen
    } else if (file & our_pawns).has_set_bit() && (file & their_pawns).has_set_bit() {
        Closed
    } else {
        SemiClosed
    }
}

impl StaticallyNamedEntity for LiTEval {
    fn static_short_name() -> &'static str
    where
        Self: Sized,
    {
        "LiTE"
    }

    fn static_long_name() -> String
    where
        Self: Sized,
    {
        "Chess LiTE -- Linear Tuned Eval for Chess".to_string()
    }

    fn static_description() -> String
    where
        Self: Sized,
    {
        "A classical evaluation for chess, based on piece square tables".to_string()
    }
}

impl LiTEval {
    fn psqt(pos: &Chessboard) -> PhasedScore {
        let mut res = PhasedScore::default();
        for color in Color::iter() {
            for piece in UncoloredChessPiece::pieces() {
                for square in pos.colored_piece_bb(color, piece).ones() {
                    let square_idx = square.flip_if(color == White).bb_idx();
                    res += PSQTS[piece as usize][square_idx];
                }
            }
            res = -res;
        }
        res
    }

    fn bishop_pair(pos: &Chessboard, color: Color) -> PhasedScore {
        if pos.colored_piece_bb(color, Bishop).more_than_one_bit_set() {
            BISHOP_PAIR
        } else {
            PhasedScore::default()
        }
    }

    fn pawn_shield(pos: &Chessboard, color: Color) -> PhasedScore {
        let our_pawns = pos.colored_piece_bb(color, Pawn);
        let king_square = pos.king_square(color);
        PAWN_SHIELDS[pawn_shield_idx(our_pawns, king_square, color)]
    }

    fn pawns(pos: &Chessboard, color: Color) -> PhasedScore {
        let our_pawns = pos.colored_piece_bb(color, Pawn);
        let their_pawns = pos.colored_piece_bb(color.other(), Pawn);
        let mut score = PhasedScore::default();

        for square in our_pawns.ones() {
            let normalized_square = square.flip_if(color == White);
            let in_front =
                (A_FILE << (square.flip_if(color == Black).bb_idx() + 8)).flip_if(color == Black);
            let blocking = in_front | in_front.west() | in_front.east();
            if (in_front & our_pawns).is_zero() && (blocking & their_pawns).is_zero() {
                score += PASSED_PAWNS[normalized_square.bb_idx()];
            }
        }
        for piece in UncoloredChessPiece::pieces() {
            let bb = pos.colored_piece_bb(color, piece);
            let pawn_attacks = our_pawns.pawn_attacks(color);
            let protected_by_pawns = pawn_attacks & bb;
            score += PAWN_PROTECTION[piece as usize] * protected_by_pawns.num_ones();
            let attacked_by_pawns = pawn_attacks & pos.colored_piece_bb(color.other(), piece);
            score += PAWN_ATTACKS[piece as usize] * attacked_by_pawns.num_ones();
        }

        score
    }

    fn rook_and_king(pos: &Chessboard, color: Color) -> PhasedScore {
        let mut score = PhasedScore::default();
        let our_pawns = pos.colored_piece_bb(color, Pawn);
        let their_pawns = pos.colored_piece_bb(color.other(), Pawn);
        // Rooks on (semi)open/closed files (semi-closed files are handled by adjusting the base rook values during tuning)
        let rooks = pos.colored_piece_bb(color, Rook);
        for rook in rooks.ones() {
            match file_openness(rook.file(), our_pawns, their_pawns) {
                Open => {
                    score += ROOK_OPEN_FILE;
                }
                SemiOpen => {
                    score += ROOK_SEMIOPEN_FILE;
                }
                SemiClosed => {}
                Closed => {
                    score += ROOK_CLOSED_FILE;
                }
            }
        }
        // King on (semi)open/closed file
        let king_square = pos.king_square(color);
        let king_file = king_square.file();
        match file_openness(king_file, our_pawns, their_pawns) {
            Open => {
                score += KING_OPEN_FILE;
            }
            SemiOpen => {
                score += KING_SEMIOPEN_FILE;
            }
            SemiClosed => {}
            Closed => {
                score += KING_CLOSED_FILE;
            }
        }
        score
    }

    fn undo_incremental_psqt(
        &mut self,
        current_pos: &Chessboard,
        mov: ChessMove,
        old_pos: &Chessboard,
    ) {
        debug_assert_eq!(self.psqt_score, Self::psqt(current_pos));
        if mov != ChessMove::default() {
            debug_assert_eq!(
                &old_pos.make_move(mov).unwrap(),
                current_pos,
                " {old_pos} {0} {current_pos} {mov}",
                { old_pos.make_move(mov).unwrap() }
            );
            let (psqt_delta, phase_delta) = Self::psqt_delta(old_pos, mov, current_pos);
            self.psqt_score -= psqt_delta;
            self.phase -= phase_delta;
        }
        debug_assert_eq!(self.psqt_score, Self::psqt(old_pos));
        debug_assert_eq!(self.phase, Self::eval_from_scratch(old_pos).0.phase);
    }

    fn psqt_delta(
        old_pos: &Chessboard,
        mov: ChessMove,
        new_pos: &Chessboard,
    ) -> (PhasedScore, PhaseType) {
        let moving_player = old_pos.active_player();
        // the current player has been flipped
        let mut delta = PhasedScore::default();
        let mut phase_delta = PhaseType::default();
        let piece = mov.uncolored_piece();
        let captured = mov.captured(old_pos);
        let src_square = mov.src_square().flip_if(moving_player == White).bb_idx();
        let dest_square = mov.dest_square().flip_if(moving_player == White);
        delta -= PSQTS[piece as usize][src_square];
        if mov.is_castle() {
            let side = mov.castle_side();
            delta += PSQTS[King as usize][new_pos
                .king_square(moving_player)
                .flip_if(moving_player == White)
                .bb_idx()];
            // since PSQTs are player-relative, castling always takes place on the 0th rank
            let rook_dest_square = ChessSquare::from_rank_file(7, side.rook_dest_file());
            let rook_start_square =
                ChessSquare::from_rank_file(7, old_pos.rook_start_file(moving_player, side));
            delta += PSQTS[Rook as usize][rook_dest_square.bb_idx()];
            delta -= PSQTS[Rook as usize][rook_start_square.bb_idx()];
        } else if mov.promo_piece() == Empty {
            delta += PSQTS[piece as usize][dest_square.bb_idx()];
        } else {
            delta += PSQTS[mov.promo_piece() as usize][dest_square.bb_idx()];
            phase_delta += PIECE_PHASE[mov.promo_piece() as usize];
        }
        if mov.is_ep() {
            delta += PSQTS[Pawn as usize][dest_square.flip().south_unchecked().bb_idx()];
        } else if captured != Empty {
            // capturing a piece increases our score by the piece's psqt value from the opponent's point of view
            delta += PSQTS[captured as usize][dest_square.flip().bb_idx()];
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

    fn finalize(score: PhasedScore, phase: PhaseType, color: Color) -> Score {
        let score = score.taper(phase, 24);
        TEMPO
            + match color {
                White => score,
                Black => -score,
            }
    }

    fn eval_from_scratch(pos: &Chessboard) -> (LiTEval, PhasedScore) {
        let mut state = LiTEval::default();
        state.hash = pos.zobrist_hash();
        state.psqt_score = Self::psqt(&pos);
        let mut score = PhasedScore::default();
        let mut phase = 0;

        for color in Color::iter() {
            score += Self::bishop_pair(pos, color);
            score += Self::pawns(pos, color);
            score += Self::pawn_shield(pos, color);
            score += Self::rook_and_king(pos, color);

            score = -score;
        }
        for piece in UncoloredChessPiece::non_king_pieces() {
            phase += pos.piece_bb(piece).num_ones() as isize * PIECE_PHASE[piece as usize];
        }
        state.phase = phase;
        score += state.psqt_score;
        (state, score)
    }
}

impl Eval<Chessboard> for LiTEval {
    fn eval(&mut self, pos: &Chessboard) -> Score {
        let (state, score) = Self::eval_from_scratch(pos);
        *self = state;
        Self::finalize(score, self.phase, pos.active_player())
    }

    fn eval_incremental(
        &mut self,
        old_pos: &Chessboard,
        mov: ChessMove,
        new_pos: &Chessboard,
        ply: usize,
    ) -> Score {
        // TODO: Store stack of eval states indexed by ply
        // Eval isn't called on nodes with a PV node TT entry, so it's possible that eval was not called on the previous
        // position. Zobrist hash collisions should be rare enough not to matter, since they would require the previous
        // position to be a PV node with the same hash as the current node
        if old_pos.zobrist_hash() != self.hash {
            return self.eval(new_pos);
        } else if mov != ChessMove::default() {
            // null moves are encoded as a1a1, but it's possible that there's a "captured" piece on a1
            debug_assert_eq!(
                Self::psqt(old_pos),
                self.psqt_score,
                "{0} {1} {old_pos} {new_pos} {mov}",
                Self::psqt(old_pos),
                self.psqt_score
            );
            debug_assert_eq!(&old_pos.make_move(mov).unwrap(), new_pos);
            let (psqt_delta, phase_delta) = Self::psqt_delta(old_pos, mov, new_pos);
            self.psqt_score += psqt_delta;
            self.phase += phase_delta;
            debug_assert_eq!(
                self.psqt_score,
                Self::psqt(new_pos),
                "{0} {1} {2} {old_pos} {new_pos} {mov}",
                self.psqt_score,
                Self::psqt(new_pos),
                Self::psqt_delta(old_pos, mov, new_pos).0,
            );
        }
        self.hash = new_pos.zobrist_hash();
        let mut score = PhasedScore::default();
        for color in Color::iter() {
            score += Self::bishop_pair(new_pos, color);
            score += Self::pawns(new_pos, color);
            score += Self::pawn_shield(new_pos, color);
            score += Self::rook_and_king(new_pos, color);
            score = -score;
        }
        score += self.psqt_score;
        debug_assert_eq!(
            score,
            Self::eval_from_scratch(new_pos).1,
            "{score} {} {old_pos} {new_pos} {mov}",
            Self::eval_from_scratch(new_pos).1
        );
        Self::finalize(score, self.phase, new_pos.active_player())
    }

    // fn undo_move(&mut self, current_pos: &Chessboard, mov: ChessMove, previous_pos: &Chessboard) {
    //     debug_assert_eq!(
    //         self.hash,
    //         current_pos.zobrist_hash(),
    //         "{previous_pos} {current_pos}"
    //     );
    //     self.undo_incremental_psqt(current_pos, mov, previous_pos);
    //     self.hash = previous_pos.zobrist_hash();
    // }
}
