use derive_more::Display;
use gears::games::chess::ChessColor;
use gears::games::chess::ChessColor::Black;
use gears::games::chess::pieces::NUM_CHESS_PIECES;
use gears::games::chess::squares::{A_FILE_NUM, ChessSquare, H_FILE_NUM, NUM_SQUARES};
use gears::general::bitboards::chessboard::ChessBitboard;
use gears::general::bitboards::{Bitboard, KnownSizeBitboard};
use gears::general::squares::RectangularCoordinates;
use strum_macros::EnumIter;

pub mod king_gambot;
pub mod lite;
pub mod lite_values;
pub mod material_only;
pub mod piston;

/// Has to be in the same order as the `FileOpenness` in `lite`.
/// `SemiClosed` is last because it doesn't get counted.
#[derive(Debug, Eq, PartialEq, Copy, Clone, EnumIter, Display)]
#[must_use]
pub enum FileOpenness {
    Open,
    Closed,
    SemiOpen,
    SemiClosed,
}

pub type DiagonalOpenness = FileOpenness;

pub const CHESS_PHASE_VALUES: [usize; NUM_CHESS_PIECES] = [0, 1, 1, 2, 4, 0];

pub const NUM_PSQT_FEATURES: usize = NUM_CHESS_PIECES * NUM_SQUARES;

pub const NUM_PAWN_SHIELD_CONFIGURATIONS: usize = (1 << 6) + (1 << 4) + (1 << 4);

pub const NUM_PAWN_CENTER_CONFIGURATIONS: usize = 1 << 6;

pub const PAWN_SHIELD_SHIFT: [usize; NUM_SQUARES] = {
    let mut res = [0; NUM_SQUARES];
    let mut square = 0;
    while square < 64 {
        let mut entry = if square % 8 == 0 { square + 8 } else { square + 7 };
        if entry > 63 {
            entry = 63;
        }
        res[square] = entry;
        square += 1;
    }
    res
};

pub fn pawn_shield_idx(mut pawns: ChessBitboard, mut king: ChessSquare, color: ChessColor) -> usize {
    if color == Black {
        king = king.flip();
        pawns = pawns.flip_up_down();
    }
    let mut bb = pawns >> PAWN_SHIELD_SHIFT[king.bb_idx()];
    // TODO: pext if available
    let file = king.file();
    if file == A_FILE_NUM || file == H_FILE_NUM {
        bb &= ChessBitboard::from_raw(0x303);
        let mut pattern = (bb.raw() | (bb.raw() >> (8 - 2))) as usize & 0x3f;
        if pattern.count_ones() > 2 {
            pattern = 0b11_11;
        }
        if file == A_FILE_NUM { (1 << 6) + pattern } else { (1 << 6) + (1 << 4) + pattern }
    } else {
        bb &= ChessBitboard::from_raw(0x707);
        let mut pattern = (bb.raw() | (bb.raw() >> (8 - 3))) as usize & 0x7f;
        if pattern.count_ones() > 3 {
            pattern = 0b111_111;
        }
        pattern
    }
}

pub fn pawn_advanced_center_idx(mut pawns: ChessBitboard, color: ChessColor) -> usize {
    if color == Black {
        pawns = pawns.flip_up_down();
    }
    let pawns = pawns.0;
    ((pawns >> (24 + 2) & 0xf) | (pawns >> (32 + 3 - 4) & (0x3 << 4))) as usize
}

pub fn pawn_passive_center_idx(mut pawns: ChessBitboard, color: ChessColor) -> usize {
    if color == Black {
        pawns = pawns.flip_up_down();
    }
    let pawns = pawns.0;
    ((pawns >> (16 + 2) & 0xf) | (pawns >> (24 + 3 - 4) & (0x3 << 4))) as usize
}

/// Returns the bitboard of squares on which our pawn can be stopped from promoting by our opponent's king
/// (a.k.a. the square rule), assuming it's our turn to move.
/// Assumes that our pawn is white and the opponent's king is black.
const fn reachable_pawns(king_sq: usize) -> ChessBitboard {
    // Unfortunately, the ChessSquare methods are from traits and therefore not `const`.
    let mut res = 0;
    let king_rank = king_sq / 8;
    let king_file = king_sq % 8;
    let mut sq = 0;
    while sq < 64 {
        let mut rank = sq / 8;
        if rank <= 1 {
            rank = 2;
        }
        let file = sq % 8;
        let file_diff = king_file.abs_diff(file);
        if file_diff < 8 - rank && king_rank >= rank {
            res |= 1 << sq;
        }
        sq += 1;
    }
    ChessBitboard::new(res)
}

pub const REACHABLE_PAWNS: [ChessBitboard; 64] = {
    let mut res = [ChessBitboard::new(0); 64];
    let mut i = 0;
    while i < 64 {
        res[i] = reachable_pawns(i);
        i += 1;
    }
    res
};

pub const FLANK: [ChessBitboard; 8] = {
    let queenside = ChessBitboard::new(0x0707_0707_0707_0707);
    let kingside = ChessBitboard::new(queenside.0 << 5);
    let center = ChessBitboard::new((queenside.0 | (queenside.0 << 1)) << 2);
    [queenside, queenside, queenside, center, center, kingside, kingside, kingside]
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::Eval;
    use crate::eval::chess::lite::LiTEval;
    use crate::eval::chess::material_only::MaterialOnlyEval;
    use crate::eval::chess::piston::PistonEval;

    use gears::games::chess::ChessColor::White;
    use gears::games::chess::pieces::ChessPieceType::Pawn;
    use gears::games::chess::{ChessColor, Chessboard};
    use gears::games::{Color, DimT};
    use gears::general::bitboards::RawBitboard;
    use gears::general::board::{BitboardBoard, Board};
    use gears::score::Score;

    #[test]
    fn pawn_shield_startpos_test() {
        let pos = Chessboard::default();
        let pawns = pos.piece_bb(Pawn);
        let white = pawn_shield_idx(pawns, pos.king_square(White), White);
        let black = pawn_shield_idx(pawns, pos.king_square(Black), Black);
        assert_eq!(white, black);
        assert_eq!(white, 0b111);
        assert_eq!(pawn_shield_idx(pawns, pos.king_square(White), Black), 0);
        assert_eq!(pawn_shield_idx(pawns, pos.king_square(Black), White), 0);
        let a = pawn_shield_idx(pos.empty_bb(), pos.king_square(White), White);
        let b = pawn_shield_idx(pos.empty_bb(), pos.king_square(Black), Black);
        assert_eq!(a, b);
        assert_eq!(a, 0b111_000);
        for file in 0..8 {
            let a = pawn_shield_idx(pawns, ChessSquare::from_rank_file(0, file), White);
            let b = pawn_shield_idx(pawns, ChessSquare::from_rank_file(7, file), Black);
            assert_eq!(a, b);
            if file == 0 {
                assert_eq!(a, 0b11 + (1 << 6));
            } else if file == 7 {
                assert_eq!(a, 0b11 + (1 << 6) + (1 << 4));
            } else {
                assert_eq!(a, 0b111);
            }
        }
    }

    #[test]
    fn pawn_shield_kiwipete_test() {
        let pos = Chessboard::from_name("kiwipete").unwrap();
        let white = pawn_shield_idx(pos.piece_bb(Pawn), pos.king_square(White), White);
        let black = pawn_shield_idx(pos.piece_bb(Pawn), pos.king_square(Black), Black);
        assert_eq!(white, 0b100);
        assert_eq!(black, 0b010_101);
    }

    fn expected_pawn_shield_idx(mut pawns: ChessBitboard, mut king: ChessSquare, color: ChessColor) -> usize {
        if color == Black {
            pawns = pawns.flip_up_down();
            king = king.flip();
        }
        let mut res = 0;

        let file_deltas = if king.file() % 8 == 0 {
            res += 1 << 6;
            vec![0, 1]
        } else if king.file() % 8 == 7 {
            res += (1 << 6) + (1 << 4);
            vec![-1, 0]
        } else {
            vec![-1, 0, 1]
        };
        let base = res;
        let mut num_pawns = 0;
        for (i, delta_file) in file_deltas.iter().enumerate() {
            for delta_rank in [1, 2] {
                let file = king.file() as isize + delta_file;
                let rank = king.rank() as usize + delta_rank;
                if !(0..8).contains(&file) || rank >= 8 {
                    continue;
                }
                let square = ChessSquare::from_rank_file(rank as DimT, file as DimT);
                if pawns.is_bit_set_at(square.bb_idx()) {
                    res += 1 << (i + (delta_rank - 1) * file_deltas.len());
                    num_pawns += 1;
                }
            }
        }
        if num_pawns > file_deltas.len() {
            return base + (1 << (2 * file_deltas.len())) - 1;
        }
        res
    }

    #[test]
    fn pawn_shield_bench_pos_test() {
        for pos in Chessboard::bench_positions() {
            for square in ChessSquare::iter() {
                for color in ChessColor::iter() {
                    let pawns = pos.col_piece_bb(color, Pawn);
                    let actual = pawn_shield_idx(pawns, square, color);
                    let expected = expected_pawn_shield_idx(pawns, square, color);
                    assert_eq!(actual, expected);
                    assert!(actual <= NUM_PAWN_SHIELD_CONFIGURATIONS, "{actual}");
                }
            }
        }
    }

    fn generic_eval_test<E: Eval<Chessboard> + Default>() {
        let score = E::default().eval(&Chessboard::default(), 0, White);
        assert!(score.abs() <= Score(25));
        assert!(score >= Score(0));
        let score = E::default().eval(&Chessboard::from_name("lucena").unwrap(), 0, White);
        assert!(score >= Score(100));
    }

    #[test]
    fn simple_eval_test() {
        generic_eval_test::<MaterialOnlyEval>();
        generic_eval_test::<PistonEval>();
        generic_eval_test::<LiTEval>();
    }
}
