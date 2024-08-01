#[cfg(test)]

mod common {
    use crate::general::common::*;

    #[test]
    fn test_pop_lsb64() {
        let mut i = 0x123;
        assert_eq!(pop_lsb64(&mut i), 0);
        assert_eq!(i, 0x122);
        assert_eq!(pop_lsb64(&mut i), 1);
        assert_eq!(i, 0x120);
        assert_eq!(pop_lsb64(&mut i), 5);
        assert_eq!(i, 0x100);
        assert_eq!(pop_lsb64(&mut i), 8);
        assert_eq!(i, 0);
    }

    #[test]
    fn test_pop_lsb128() {
        let mut i: u128 = 0xa800_0000_0000_0000_0000_0000_0001;
        assert_eq!(pop_lsb128(&mut i), 0);
        assert_eq!(i, 0xa800_0000_0000_0000_0000_0000_0000);
        assert_eq!(pop_lsb128(&mut i), 13 * 8 + 3);
        assert_eq!(i, 0xa000_0000_0000_0000_0000_0000_0000);
        assert_eq!(pop_lsb128(&mut i), 13 * 8 + 5);
        assert_eq!(i, 0x8000_0000_0000_0000_0000_0000_0000);
        assert_eq!(pop_lsb128(&mut i), 13 * 8 + 7);
        assert_eq!(i, 0);
    }
}

#[cfg(test)]
mod bitboards {

    #[cfg(feature = "chess")]
    mod chessboard {
        use crate::games::chess::squares::{ChessSquare, ChessboardSize};
        use crate::general::bitboards::chess::ChessBitboard;
        use crate::general::bitboards::{Bitboard, RawBitboard};

        #[test]
        fn is_single_piece_test() {
            assert!(!ChessBitboard::from_u64(0x0).is_single_piece());
            assert!(ChessBitboard::from_u64(0x1).is_single_piece());
            assert!(ChessBitboard::from_u64(0x2).is_single_piece());
            assert!(!ChessBitboard::from_u64(0x3).is_single_piece());
            assert!(ChessBitboard::from_u64(0x4).is_single_piece());
            assert!(ChessBitboard::from_u64(0x400).is_single_piece());
            assert!(!ChessBitboard::from_u64(0x4001).is_single_piece());
        }

        #[test]
        fn trailing_zeros_test() {
            assert_eq!(ChessBitboard::from_u64(0).trailing_zeros(), 64);
            assert_eq!(ChessBitboard::from_u64(1).trailing_zeros(), 0);
            assert_eq!(ChessBitboard::from_u64(2).trailing_zeros(), 1);
            assert_eq!(ChessBitboard::from_u64(0xa).trailing_zeros(), 1);
            assert_eq!(
                ChessBitboard::from_u64(0xa0bc_00de_f000).trailing_zeros(),
                12
            );
        }

        #[test]
        fn diag_test() {
            assert_eq!(
                ChessBitboard::diag_for_sq(
                    ChessSquare::from_bb_index(0),
                    ChessboardSize::default()
                ),
                ChessBitboard::from_u64(0x8040_2010_0804_0201)
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(
                    ChessSquare::from_bb_index(1),
                    ChessboardSize::default()
                ),
                ChessBitboard::from_u64(0x80_4020_1008_0402)
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(
                    ChessSquare::from_bb_index(7),
                    ChessboardSize::default()
                ),
                ChessBitboard::from_u64(0x80)
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(
                    ChessSquare::from_bb_index(8),
                    ChessboardSize::default()
                ),
                ChessBitboard::from_u64(0x4020_1008_0402_0100)
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(
                    ChessSquare::from_bb_index(9),
                    ChessboardSize::default()
                ),
                ChessBitboard::diag_for_sq(
                    ChessSquare::from_bb_index(0),
                    ChessboardSize::default()
                )
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(
                    ChessSquare::from_bb_index(15),
                    ChessboardSize::default()
                ),
                ChessBitboard::from_u64(0x8040)
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(
                    ChessSquare::from_bb_index(10),
                    ChessboardSize::default()
                ),
                ChessBitboard::diag_for_sq(
                    ChessSquare::from_bb_index(1),
                    ChessboardSize::default()
                )
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(
                    ChessSquare::from_bb_index(12),
                    ChessboardSize::default()
                ),
                ChessBitboard::diag_for_sq(
                    ChessSquare::from_bb_index(3),
                    ChessboardSize::default()
                )
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(
                    ChessSquare::from_bb_index(17),
                    ChessboardSize::default()
                ),
                ChessBitboard::diag_for_sq(
                    ChessSquare::from_bb_index(8),
                    ChessboardSize::default()
                )
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(
                    ChessSquare::from_bb_index(42),
                    ChessboardSize::default()
                ),
                ChessBitboard::diag_for_sq(
                    ChessSquare::from_bb_index(33),
                    ChessboardSize::default()
                )
            );
        }

        #[test]
        fn anti_diag_test() {
            assert_eq!(
                ChessBitboard::anti_diag_for_sq(
                    ChessSquare::from_bb_index(0),
                    ChessboardSize::default()
                ),
                ChessBitboard::from_u64(1)
            );
            assert_eq!(
                ChessBitboard::anti_diag_for_sq(
                    ChessSquare::from_bb_index(7),
                    ChessboardSize::default()
                ),
                ChessBitboard::from_u64(0x0102_0408_1020_4080)
            );
            assert_eq!(
                ChessBitboard::anti_diag_for_sq(
                    ChessSquare::from_bb_index(14),
                    ChessboardSize::default()
                ),
                ChessBitboard::anti_diag_for_sq(
                    ChessSquare::from_bb_index(7),
                    ChessboardSize::default()
                )
            );
            assert_eq!(
                ChessBitboard::anti_diag_for_sq(
                    ChessSquare::from_bb_index(8),
                    ChessboardSize::default()
                ),
                ChessBitboard::from_u64(0x0102)
            );
            assert_eq!(
                ChessBitboard::anti_diag_for_sq(
                    ChessSquare::from_bb_index(15),
                    ChessboardSize::default()
                ),
                ChessBitboard::from_u64(0x0204_0810_2040_8000)
            );
            assert_eq!(
                ChessBitboard::anti_diag_for_sq(
                    ChessSquare::from_bb_index(42),
                    ChessboardSize::default()
                ),
                ChessBitboard::anti_diag_for_sq(
                    ChessSquare::from_bb_index(35),
                    ChessboardSize::default()
                )
            );
        }

        #[test]
        fn flip_left_right_test() {
            assert_eq!(
                ChessBitboard::from_u64(0).flip_left_right(0),
                ChessBitboard::from_u64(0)
            );
            assert_eq!(
                ChessBitboard::from_u64(1).flip_left_right(0),
                ChessBitboard::from_u64(0x80)
            );
            for i in 0..7 {
                assert_eq!(
                    (ChessBitboard::from_u64(0x0003_4010_00e0).flip_left_right(i) >> (8 * i)).0
                        & 0xff,
                    (ChessBitboard::from_u64(0x00c0_0208_0007) >> (8 * i)).0 & 0xff
                );
            }
            assert_eq!(
                ChessBitboard::from_u64(0xffff_ffff_ffff_fffe).flip_left_right(0),
                ChessBitboard::from_u64(0xffff_ffff_ffff_ff7f & 0xff)
            );
        }

        #[test]
        fn flip_up_down_test() {
            assert_eq!(
                ChessBitboard::from_u64(0).flip_up_down(),
                ChessBitboard::from_u64(0)
            );
            assert_eq!(
                ChessBitboard::from_u64(1).flip_up_down(),
                ChessBitboard::from_u64(0x0100_0000_0000_0000)
            );
            assert_eq!(
                ChessBitboard::from_u64(0x0340_1000_e000_00ac).flip_up_down(),
                ChessBitboard::from_u64(0xac00_00e0_0010_4003)
            );
            assert_eq!(
                ChessBitboard::from_u64(0xffff_ffff_ffff_fffe).flip_up_down(),
                ChessBitboard::from_u64(0xfeff_ffff_ffff_ffff)
            );
        }
    }

    mod extended_board {
        use crate::games::mnk::MnkBitboard;
        use crate::games::{Height, Width};
        use crate::general::bitboards::{Bitboard, ExtendedRawBitboard, RawBitboard};
        use crate::general::squares::{GridCoordinates, GridSize, RectangularCoordinates};

        const LARGER_THAN_64_BIT: u128 = 1 << 64;

        #[test]
        fn is_single_piece_test() {
            assert!(!ExtendedRawBitboard(0x0).is_single_piece());
            assert!(ExtendedRawBitboard(0x1).is_single_piece());
            assert!(ExtendedRawBitboard(0x2).is_single_piece());
            assert!(!ExtendedRawBitboard(0x3).is_single_piece());
            assert!(ExtendedRawBitboard(0x4).is_single_piece());
            assert!(ExtendedRawBitboard(0x400).is_single_piece());
            assert!(!ExtendedRawBitboard(0x4001).is_single_piece());
            assert!(ExtendedRawBitboard(LARGER_THAN_64_BIT).is_single_piece());
            assert!(
                ExtendedRawBitboard(0x200_0000_0000_0000_0000_0000_0000_0000).is_single_piece()
            );
        }

        #[test]
        fn trailing_zeros_test() {
            assert_eq!(ExtendedRawBitboard(0).trailing_zeros(), 128);
            assert_eq!(ExtendedRawBitboard(1).trailing_zeros(), 0);
            assert_eq!(ExtendedRawBitboard(2).trailing_zeros(), 1);
            assert_eq!(ExtendedRawBitboard(0xa).trailing_zeros(), 1);
            assert_eq!(ExtendedRawBitboard(0xa0bc_00de_f000).trailing_zeros(), 12);
            assert_eq!(
                ExtendedRawBitboard(0xa0bc_00de_f000 + LARGER_THAN_64_BIT).trailing_zeros(),
                12
            );
            assert_eq!((!ExtendedRawBitboard(0)).trailing_zeros(), 0);
            assert_eq!((!ExtendedRawBitboard(0) << 2).trailing_zeros(), 2);
        }

        #[test]
        fn diag_test() {
            let size = GridSize::new(Height(1), Width(2));
            assert_eq!(
                MnkBitboard::diag_for_sq(GridCoordinates::from_row_column(0, 0), size,)
                    & MnkBitboard::from_uint(0b11, size),
                MnkBitboard::from_uint(1, size)
            );
            let size = GridSize::new(Height(3), Width(2));
            assert_eq!(
                MnkBitboard::diag_for_sq(GridCoordinates::from_row_column(0, 0), size,),
                MnkBitboard::from_uint(0b00_1001, size)
            );
            assert_eq!(
                MnkBitboard::diag_for_sq(GridCoordinates::from_row_column(0, 1), size,),
                MnkBitboard::from_uint(0b10, size)
            );
            let size = GridSize::new(Height(11), Width(8));
            assert_eq!(
                MnkBitboard::diag_for_sq(GridCoordinates::from_row_column(0, 7), size,),
                MnkBitboard::from_uint(0x80, size)
            );
            let size = GridSize::new(Height(9), Width(8));
            assert_eq!(
                MnkBitboard::diag_for_sq(GridCoordinates::from_row_column(1, 0), size,)
                    & MnkBitboard::from_uint(u64::MAX as u128, size),
                MnkBitboard::from_uint(0x4020_1008_0402_0100, size)
            );
            let size = GridSize::new(Height(7), Width(3));
            assert_eq!(
                MnkBitboard::diag_for_sq(GridCoordinates::from_row_column(0, 1), size,),
                MnkBitboard::from_uint(0b100_010, size)
            );
            for width in 1..12 {
                for square in width + 1..9 * width {
                    let prev = square - width - 1;
                    if square % width == 0 {
                        continue;
                    }
                    let size = GridSize::new(Height(9), Width(width));
                    assert_eq!(
                        MnkBitboard::diag_for_sq(
                            GridCoordinates::from_row_column(square / width, square % width),
                            size,
                        ),
                        MnkBitboard::diag_for_sq(
                            GridCoordinates::from_row_column(prev / width, prev % width),
                            size
                        )
                    );
                }
            }
        }

        #[test]
        fn anti_diag_test() {
            let size = GridSize::new(Height(3), Width(1));
            assert_eq!(
                MnkBitboard::anti_diag_for_sq(GridCoordinates::from_row_column(0, 0), size,),
                MnkBitboard::from_uint(1, size)
            );
            let size = GridSize::new(Height(4), Width(1));
            assert_eq!(
                MnkBitboard::anti_diag_for_sq(GridCoordinates::from_row_column(1, 0), size,),
                MnkBitboard::from_uint(0b10, size)
            );
            let size = GridSize::new(Height(3), Width(2));
            assert_eq!(
                MnkBitboard::anti_diag_for_sq(GridCoordinates::from_row_column(0, 1), size,)
                    & MnkBitboard::from_uint(0b111_111, size),
                MnkBitboard::from_uint(0b0110, size)
            );

            for width in 1..12 {
                for square in width - 1..9 * width {
                    let prev = square - (width - 1);
                    if prev % width == 0 {
                        continue;
                    }
                    let size = GridSize::new(Height(9), Width(width));
                    assert_eq!(
                        MnkBitboard::anti_diag_for_sq(
                            GridCoordinates::from_row_column(square / width, square % width),
                            size,
                        ),
                        MnkBitboard::anti_diag_for_sq(
                            GridCoordinates::from_row_column(prev / width, prev % width),
                            size,
                        )
                    );
                }
            }
        }

        #[test]
        fn flip_left_right_test() {
            let size = GridSize::new(Height(4), Width(3));
            assert_eq!(
                MnkBitboard::from_uint(0, size).flip_left_right(0),
                MnkBitboard::from_uint(0, size)
            );
            let size = GridSize::chess();
            assert_eq!(
                MnkBitboard::from_uint(1, size).flip_left_right(0),
                MnkBitboard::from_uint(0b1000_0000, size)
            );
            let size = GridSize::new(Height(12), Width(2));
            assert_eq!(
                MnkBitboard::from_uint(0x02_34e1, size).flip_left_right(0),
                MnkBitboard::from_uint(0x01_38d2, size)
            );
            let size = GridSize::new(Height(7), Width(3));
            assert_eq!(
                MnkBitboard::from_uint(0b101_001_011_110_111_001, size).flip_left_right(0),
                MnkBitboard::from_uint(0b101_100_110_011_111_100, size)
            );
            let size = GridSize::tictactoe();
            assert_eq!(
                MnkBitboard::from_uint(0x4924_9249_2492_4924_9249_2492_4924_9249, size)
                    .flip_left_right(0),
                MnkBitboard::from_uint(0x2492_4924_9249_2492_4924_9249_2492_4924, size)
            );
        }

        #[test]
        fn flip_up_down_test() {
            let size = GridSize::new(Height(7), Width(3));
            assert_eq!(
                MnkBitboard::from_uint(0, size).flip_up_down(),
                MnkBitboard::from_uint(0, size)
            );
            let size = GridSize::new(Height(2), Width(9));
            assert_eq!(
                MnkBitboard::from_uint(1, size).flip_up_down(),
                MnkBitboard::from_uint(0x200, size)
            );
            let size = GridSize::new(Height(12), Width(8));
            assert_eq!(
                MnkBitboard::from_uint(0x0340_1000_e000_00ac, size).flip_up_down(),
                MnkBitboard::from_uint(0xac00_00e0_0010_4003_0000_0000, size)
            );
            let size = GridSize::new(Height(3), Width(10));
            assert_eq!(
                MnkBitboard::from_uint(0b00110_01010_11001_11010, size).flip_up_down(),
                MnkBitboard::from_uint(0b11001_11010_00110_01010_00000_00000, size)
            );
            let size = GridSize::tictactoe();
            assert_eq!(
                MnkBitboard::from_uint(0b001_001_001, size).flip_up_down(),
                MnkBitboard::from_uint(0b001_001_001, size)
            );
            assert_eq!(
                MnkBitboard::from_uint(0x4924_9249_2492_4924_9249_2492_4924_9249, size)
                    .flip_up_down(),
                MnkBitboard::from_uint(0x4924_9249_2492_4924_9249_2492_4924_9249, size)
            );
        }
    }
}
