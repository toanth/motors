use gears::games::chess::pieces::NUM_CHESS_PIECES;
use gears::games::chess::squares::{ChessSquare, NUM_SQUARES};
use gears::games::Color;
use gears::games::Color::*;
use gears::general::bitboards::chess::ChessBitboard;
use gears::general::bitboards::Bitboard;
use std::fmt::{Display, Formatter};
use strum_macros::EnumIter;

pub mod hce;
pub mod material_only;
pub mod pst_only;

#[derive(Debug, Copy, Clone, EnumIter)]
pub enum PhaseType {
    Mg,
    Eg,
}

impl Display for PhaseType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PhaseType::Mg => write!(f, "MG"),
            PhaseType::Eg => write!(f, "EG"),
        }
    }
}

/// Has to be in the same order as the FileOpenness in hce.rs.
/// `SemiClosed` is last because it doesn't get counted.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum FileOpenness {
    Open,
    Closed,
    SemiOpen,
    SemiClosed,
}

const NUM_PHASES: usize = 2;
const CHESS_PHASE_VALUES: [usize; NUM_CHESS_PIECES] = [0, 1, 1, 2, 4, 0];

const NUM_PSQT_FEATURES: usize = NUM_CHESS_PIECES * NUM_SQUARES;

pub const NUM_PAWN_SHIELD_CONFIGURATIONS: usize = (1 << 6) + (1 << 4) + (1 << 4);

pub const PAWN_SHIELD_SHIFT: [usize; NUM_SQUARES] = {
    let mut res = [0; NUM_SQUARES];
    let mut square = 0;
    while square < 64 {
        let mut entry = if square % 8 == 0 {
            square + 8
        } else {
            square + 7
        };
        if entry > 63 {
            entry = 63;
        }
        res[square] = entry;
        square += 1;
    }
    res
};

pub fn pawn_shield_idx(mut pawns: ChessBitboard, mut king: ChessSquare, color: Color) -> usize {
    if color == Black {
        king = king.flip();
        pawns = pawns.flip_up_down();
    }
    let mut bb = pawns >> PAWN_SHIELD_SHIFT[king.idx()];
    // TODO: pext if available
    let file = king.file();
    if file == 0 {
        bb &= ChessBitboard::from_u64(0x303);
        let base_idx = (bb.0 | (bb.0 >> (8 - 2))) as usize & 0x3f;
        base_idx + (1 << 6)
    } else if file == 7 {
        bb &= ChessBitboard::from_u64(0x303);
        let base_idx = (bb.0 | (bb.0 >> (8 - 2))) as usize & 0x3f;
        base_idx + (1 << 6) + (1 << 4)
    } else {
        bb &= ChessBitboard::from_u64(0x707);
        (bb.0 | (bb.0 >> (8 - 3))) as usize & 0x7f
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gears::games::chess::pieces::UncoloredChessPiece::Pawn;
    use gears::games::chess::Chessboard;
    use gears::games::{Board, DimT};
    use gears::general::bitboards::RawBitboard;
    use strum::IntoEnumIterator;

    #[test]
    pub fn pawn_shield_startpos_test() {
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
        assert_eq!(a, 0b111000);
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
    pub fn pawn_shield_kiwipete_test() {
        let pos = Chessboard::from_name("kiwipete").unwrap();
        let white = pawn_shield_idx(pos.piece_bb(Pawn), pos.king_square(White), White);
        let black = pawn_shield_idx(pos.piece_bb(Pawn), pos.king_square(Black), Black);
        assert_eq!(white, 0b100);
        assert_eq!(black, 0b010101);
    }

    fn expected_pawn_shield_idx(
        mut pawns: ChessBitboard,
        mut king: ChessSquare,
        color: Color,
    ) -> usize {
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
        for (i, delta_file) in file_deltas.iter().enumerate() {
            for delta_rank in [1, 2] {
                let file = king.file() as isize + delta_file;
                let rank = king.rank() as usize + delta_rank;
                if file < 0 || file >= 8 || rank >= 8 {
                    continue;
                }
                let square = ChessSquare::from_rank_file(rank as DimT, file as DimT);
                if pawns.is_bit_set_at(square.idx()) {
                    res += 1 << (i + (delta_rank - 1) * file_deltas.len());
                }
            }
        }
        res
    }

    #[test]
    pub fn pawn_shield_bench_pos_test() {
        for pos in Chessboard::bench_positions() {
            for square in ChessSquare::iter() {
                for color in Color::iter() {
                    let _fen = pos.as_fen();
                    let pawns = pos.colored_piece_bb(color, Pawn);
                    let actual = pawn_shield_idx(pawns, square, color);
                    let expected = expected_pawn_shield_idx(pawns, square, color);
                    assert_eq!(actual, expected);
                    assert!(actual <= NUM_PAWN_SHIELD_CONFIGURATIONS, "{actual}");
                }
            }
        }
    }
}
