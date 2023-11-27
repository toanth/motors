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

mod bitboards {

    mod chessboard {
        use crate::games::chess::squares::{ChessSquare, ChessboardSize};
        use crate::general::bitboards::{
            // flip_left_right_chessboard, flip_up_down_chessboard, get_file_chessboard,
            // get_rank_chessboard,
            Bitboard,
            ChessBitboard,
        };

        #[test]
        fn is_single_piece_test() {
            assert!(!ChessBitboard(0x0).is_single_piece());
            assert!(ChessBitboard(0x1).is_single_piece());
            assert!(ChessBitboard(0x2).is_single_piece());
            assert!(!ChessBitboard(0x3).is_single_piece());
            assert!(ChessBitboard(0x4).is_single_piece());
            assert!(ChessBitboard(0x400).is_single_piece());
            assert!(!ChessBitboard(0x4001).is_single_piece());
        }

        #[test]
        fn trailing_zeros_test() {
            assert_eq!(ChessBitboard(0).trailing_zeros(), 64);
            assert_eq!(ChessBitboard(1).trailing_zeros(), 0);
            assert_eq!(ChessBitboard(2).trailing_zeros(), 1);
            assert_eq!(ChessBitboard(0xa).trailing_zeros(), 1);
            assert_eq!(ChessBitboard(0xa0bc00def000).trailing_zeros(), 12);
        }

        #[test]
        fn diag_test() {
            assert_eq!(
                ChessBitboard::diag_for_sq(ChessSquare::new(0), ChessboardSize::default()),
                ChessBitboard(0x8040201008040201)
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(ChessSquare::new(1), ChessboardSize::default()),
                ChessBitboard(0x80402010080402)
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(ChessSquare::new(7), ChessboardSize::default()),
                ChessBitboard(0x80)
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(ChessSquare::new(8), ChessboardSize::default()),
                ChessBitboard(0x4020100804020100)
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(ChessSquare::new(9), ChessboardSize::default()),
                ChessBitboard::diag_for_sq(ChessSquare::new(0), ChessboardSize::default())
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(ChessSquare::new(15), ChessboardSize::default()),
                ChessBitboard(0x8040)
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(ChessSquare::new(10), ChessboardSize::default()),
                ChessBitboard::diag_for_sq(ChessSquare::new(1), ChessboardSize::default())
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(ChessSquare::new(12), ChessboardSize::default()),
                ChessBitboard::diag_for_sq(ChessSquare::new(3), ChessboardSize::default())
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(ChessSquare::new(17), ChessboardSize::default()),
                ChessBitboard::diag_for_sq(ChessSquare::new(8), ChessboardSize::default())
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(ChessSquare::new(42), ChessboardSize::default()),
                ChessBitboard::diag_for_sq(ChessSquare::new(33), ChessboardSize::default())
            );
        }

        #[test]
        fn anti_diag_test() {
            assert_eq!(
                ChessBitboard::anti_diag_for_sq(ChessSquare::new(0), ChessboardSize::default()),
                ChessBitboard(1)
            );
            assert_eq!(
                ChessBitboard::anti_diag_for_sq(ChessSquare::new(7), ChessboardSize::default()),
                ChessBitboard(0x0102_0408_1020_4080)
            );
            assert_eq!(
                ChessBitboard::anti_diag_for_sq(ChessSquare::new(14), ChessboardSize::default()),
                ChessBitboard::anti_diag_for_sq(ChessSquare::new(7), ChessboardSize::default())
            );
            assert_eq!(
                ChessBitboard::anti_diag_for_sq(ChessSquare::new(8), ChessboardSize::default()),
                ChessBitboard(0x0102)
            );
            assert_eq!(
                ChessBitboard::anti_diag_for_sq(ChessSquare::new(15), ChessboardSize::default()),
                ChessBitboard(0x0204_0810_2040_8000)
            );
            assert_eq!(
                ChessBitboard::anti_diag_for_sq(ChessSquare::new(42), ChessboardSize::default()),
                ChessBitboard::anti_diag_for_sq(ChessSquare::new(35), ChessboardSize::default())
            );
        }

        #[test]
        fn flip_left_right_test() {
            assert_eq!(
                ChessBitboard(0).flip_left_right(ChessboardSize::default()),
                ChessBitboard(0)
            );
            assert_eq!(
                ChessBitboard(1).flip_left_right(ChessboardSize::default()),
                ChessBitboard(0x80)
            );
            assert_eq!(
                ChessBitboard(0x0003_4010_00e0).flip_left_right(ChessboardSize::default()),
                ChessBitboard(0x00c0_0208_0007)
            );
            assert_eq!(
                ChessBitboard(0xffff_ffff_ffff_fffe).flip_left_right(ChessboardSize::default()),
                ChessBitboard(0xffff_ffff_ffff_ff7f)
            );
        }

        #[test]
        fn flip_up_down_test() {
            assert_eq!(
                ChessBitboard(0).flip_up_down(ChessboardSize::default()),
                ChessBitboard(0)
            );
            assert_eq!(
                ChessBitboard(1).flip_up_down(ChessboardSize::default()),
                ChessBitboard(0x0100_0000_0000_0000)
            );
            assert_eq!(
                ChessBitboard(0x0340_1000_e000_00ac).flip_up_down(ChessboardSize::default()),
                ChessBitboard(0xac00_00e0_0010_4003)
            );
            assert_eq!(
                ChessBitboard(0xffff_ffff_ffff_fffe).flip_up_down(ChessboardSize::default()),
                ChessBitboard(0xfeff_ffff_ffff_ffff)
            );
        }
    }

    mod extended_board {
        use crate::games::{GridCoordinates, GridSize, Height, RectangularCoordinates, Width};
        use crate::general::bitboards::{Bitboard, ExtendedBitboard};

        const LARGER_THAN_64_BIT: u128 = 1 << 64;

        #[test]
        fn is_single_piece_test() {
            assert!(!ExtendedBitboard(0x0).is_single_piece());
            assert!(ExtendedBitboard(0x1).is_single_piece());
            assert!(ExtendedBitboard(0x2).is_single_piece());
            assert!(!ExtendedBitboard(0x3).is_single_piece());
            assert!(ExtendedBitboard(0x4).is_single_piece());
            assert!(ExtendedBitboard(0x400).is_single_piece());
            assert!(!ExtendedBitboard(0x4001).is_single_piece());
            assert!(ExtendedBitboard(LARGER_THAN_64_BIT).is_single_piece());
            assert!(ExtendedBitboard(0x2000000000000000000000000000000).is_single_piece());
        }

        #[test]
        fn trailing_zeros_test() {
            assert_eq!(ExtendedBitboard(0).trailing_zeros(), 128);
            assert_eq!(ExtendedBitboard(1).trailing_zeros(), 0);
            assert_eq!(ExtendedBitboard(2).trailing_zeros(), 1);
            assert_eq!(ExtendedBitboard(0xa).trailing_zeros(), 1);
            assert_eq!(ExtendedBitboard(0xa0bc00def000).trailing_zeros(), 12);
            assert_eq!(
                ExtendedBitboard(0xa0bc00def000 + LARGER_THAN_64_BIT).trailing_zeros(),
                12
            );
            assert_eq!((!ExtendedBitboard(0)).trailing_zeros(), 0);
            assert_eq!((!ExtendedBitboard(0) << 2).trailing_zeros(), 2);
        }

        #[test]
        fn diag_test() {
            assert_eq!(
                ExtendedBitboard::diag_for_sq(
                    GridCoordinates::from_row_column(0, 0),
                    GridSize::new(Height(1), Width(2))
                ) & ExtendedBitboard(0b11),
                ExtendedBitboard(1)
            );
            assert_eq!(
                ExtendedBitboard::diag_for_sq(
                    GridCoordinates::from_row_column(0, 0),
                    GridSize::new(Height(3), Width(2))
                ),
                ExtendedBitboard(0b001001)
            );
            assert_eq!(
                ExtendedBitboard::diag_for_sq(
                    GridCoordinates::from_row_column(0, 1),
                    GridSize::new(Height(3), Width(2))
                ),
                ExtendedBitboard(0b10)
            );
            assert_eq!(
                ExtendedBitboard::diag_for_sq(
                    GridCoordinates::from_row_column(0, 7),
                    GridSize::new(Height(11), Width(8))
                ),
                ExtendedBitboard(0x80)
            );
            assert_eq!(
                ExtendedBitboard::diag_for_sq(
                    GridCoordinates::from_row_column(1, 0),
                    GridSize::new(Height(9), Width(8))
                ) & ExtendedBitboard(u64::MAX as u128),
                ExtendedBitboard(0x4020100804020100)
            );
            assert_eq!(
                ExtendedBitboard::diag_for_sq(
                    GridCoordinates::from_row_column(0, 1),
                    GridSize::new(Height(7), Width(3))
                ),
                ExtendedBitboard(0b100_010)
            );
            for width in 1..12 {
                for square in width + 1..9 * width {
                    let prev = square - width - 1;
                    if square % width == 0 {
                        continue;
                    }
                    assert_eq!(
                        ExtendedBitboard::diag_for_sq(
                            GridCoordinates::from_row_column(square / width, square % width),
                            GridSize::new(Height(9), Width(width))
                        ),
                        ExtendedBitboard::diag_for_sq(
                            GridCoordinates::from_row_column(prev / width, prev % width),
                            GridSize::new(Height(9), Width(width))
                        )
                    );
                }
            }
        }

        #[test]
        fn anti_diag_test() {
            assert_eq!(
                ExtendedBitboard::anti_diag_for_sq(
                    GridCoordinates::from_row_column(0, 0),
                    GridSize::new(Height(3), Width(1))
                ),
                ExtendedBitboard(1)
            );
            assert_eq!(
                ExtendedBitboard::anti_diag_for_sq(
                    GridCoordinates::from_row_column(1, 0),
                    GridSize::new(Height(4), Width(1))
                ),
                ExtendedBitboard(0b10)
            );
            assert_eq!(
                ExtendedBitboard::anti_diag_for_sq(
                    GridCoordinates::from_row_column(0, 1),
                    GridSize::new(Height(3), Width(2))
                ) & ExtendedBitboard(0b111111),
                ExtendedBitboard(0b0110)
            );

            for width in 1..12 {
                for square in width - 1..9 * width {
                    let prev = square - (width - 1);
                    if prev % width == 0 {
                        continue;
                    }
                    assert_eq!(
                        ExtendedBitboard::anti_diag_for_sq(
                            GridCoordinates::from_row_column(square / width, square % width),
                            GridSize::new(Height(9), Width(width))
                        ),
                        ExtendedBitboard::anti_diag_for_sq(
                            GridCoordinates::from_row_column(prev / width, prev % width),
                            GridSize::new(Height(9), Width(width))
                        )
                    );
                }
            }
        }

        #[test]
        fn flip_left_right_test() {
            assert_eq!(
                ExtendedBitboard(0).flip_left_right(GridSize::new(Height(4), Width(3))),
                ExtendedBitboard(0)
            );
            assert_eq!(
                ExtendedBitboard(1).flip_left_right(GridSize::chess()),
                ExtendedBitboard(0b1000_0000)
            );
            assert_eq!(
                ExtendedBitboard(0x0234e1).flip_left_right(GridSize::new(Height(12), Width(2))),
                ExtendedBitboard(0x0138d2)
            );
            assert_eq!(
                ExtendedBitboard(0b101_001_011_110_111_001)
                    .flip_left_right(GridSize::new(Height(7), Width(3))),
                ExtendedBitboard(0b101_100_110_011_111_100)
            );
            assert_eq!(
                ExtendedBitboard(0x49249249249249249249249249249249)
                    .flip_left_right(GridSize::tictactoe()),
                ExtendedBitboard(0x24924924924924924924924924924924)
            );
        }

        #[test]
        fn flip_up_down_test() {
            assert_eq!(
                ExtendedBitboard(0).flip_up_down(GridSize::new(Height(7), Width(3))),
                ExtendedBitboard(0)
            );
            assert_eq!(
                ExtendedBitboard(1).flip_up_down(GridSize::new(Height(2), Width(9))),
                ExtendedBitboard(0x200)
            );
            assert_eq!(
                ExtendedBitboard(0x0340_1000_e000_00ac)
                    .flip_up_down(GridSize::new(Height(12), Width(8))),
                ExtendedBitboard(0xac00_00e0_0010_4003_0000_0000)
            );
            assert_eq!(
                ExtendedBitboard(0b00110_01010_11001_11010)
                    .flip_up_down(GridSize::new(Height(3), Width(10))),
                ExtendedBitboard(0b11001_11010_00110_01010_00000_00000)
            );
            assert_eq!(
                ExtendedBitboard(0b001_001_001).flip_up_down(GridSize::tictactoe()),
                ExtendedBitboard(0b001_001_001)
            );
            assert_eq!(
                ExtendedBitboard(0x49249249249249249249249249249249)
                    .flip_up_down(GridSize::tictactoe()),
                ExtendedBitboard(0x49249249249249249249249249249249)
            );
        }
    }
}
