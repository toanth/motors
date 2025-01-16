// TODO: Move into Bitboards.rs
#[cfg(test)]
mod bitboards {

    #[cfg(feature = "chess")]
    mod chessboard {
        use crate::games::chess::squares::{ChessSquare, ChessboardSize};
        use crate::general::bitboards::chessboard::ChessBitboard;
        use crate::general::bitboards::{Bitboard, KnownSizeBitboard, RawBitboard};

        #[test]
        fn is_single_piece_test() {
            assert!(!ChessBitboard::from_raw(0x0).is_single_piece());
            assert!(ChessBitboard::from_raw(0x1).is_single_piece());
            assert!(ChessBitboard::from_raw(0x2).is_single_piece());
            assert!(!ChessBitboard::from_raw(0x3).is_single_piece());
            assert!(ChessBitboard::from_raw(0x4).is_single_piece());
            assert!(ChessBitboard::from_raw(0x400).is_single_piece());
            assert!(!ChessBitboard::from_raw(0x4001).is_single_piece());
        }

        #[test]
        fn trailing_zeros_test() {
            assert_eq!(ChessBitboard::from_raw(0).num_trailing_zeros(), 64);
            assert_eq!(ChessBitboard::from_raw(1).num_trailing_zeros(), 0);
            assert_eq!(ChessBitboard::from_raw(2).num_trailing_zeros(), 1);
            assert_eq!(ChessBitboard::from_raw(0xa).num_trailing_zeros(), 1);
            assert_eq!(ChessBitboard::from_raw(0xa0bc_00de_f000).num_trailing_zeros(), 12);
        }

        #[test]
        fn diag_test() {
            assert_eq!(
                ChessBitboard::diag_for_sq(ChessSquare::from_bb_index(0), ChessboardSize::default()),
                ChessBitboard::from_raw(0x8040_2010_0804_0201)
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(ChessSquare::from_bb_index(1), ChessboardSize::default()),
                ChessBitboard::from_raw(0x80_4020_1008_0402)
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(ChessSquare::from_bb_index(7), ChessboardSize::default()),
                ChessBitboard::from_raw(0x80)
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(ChessSquare::from_bb_index(8), ChessboardSize::default()),
                ChessBitboard::from_raw(0x4020_1008_0402_0100)
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(ChessSquare::from_bb_index(9), ChessboardSize::default()),
                ChessBitboard::diag_for_sq(ChessSquare::from_bb_index(0), ChessboardSize::default())
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(ChessSquare::from_bb_index(15), ChessboardSize::default()),
                ChessBitboard::from_raw(0x8040)
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(ChessSquare::from_bb_index(10), ChessboardSize::default()),
                ChessBitboard::diag_for_sq(ChessSquare::from_bb_index(1), ChessboardSize::default())
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(ChessSquare::from_bb_index(12), ChessboardSize::default()),
                ChessBitboard::diag_for_sq(ChessSquare::from_bb_index(3), ChessboardSize::default())
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(ChessSquare::from_bb_index(17), ChessboardSize::default()),
                ChessBitboard::diag_for_sq(ChessSquare::from_bb_index(8), ChessboardSize::default())
            );
            assert_eq!(
                ChessBitboard::diag_for_sq(ChessSquare::from_bb_index(42), ChessboardSize::default()),
                ChessBitboard::diag_for_sq(ChessSquare::from_bb_index(33), ChessboardSize::default())
            );
        }

        #[test]
        fn anti_diag_test() {
            assert_eq!(
                ChessBitboard::anti_diag_for_sq(ChessSquare::from_bb_index(0), ChessboardSize::default()),
                ChessBitboard::from_raw(1)
            );
            assert_eq!(
                ChessBitboard::anti_diag_for_sq(ChessSquare::from_bb_index(7), ChessboardSize::default()),
                ChessBitboard::from_raw(0x0102_0408_1020_4080)
            );
            assert_eq!(
                ChessBitboard::anti_diag_for_sq(ChessSquare::from_bb_index(14), ChessboardSize::default()),
                ChessBitboard::anti_diag_for_sq(ChessSquare::from_bb_index(7), ChessboardSize::default())
            );
            assert_eq!(
                ChessBitboard::anti_diag_for_sq(ChessSquare::from_bb_index(8), ChessboardSize::default()),
                ChessBitboard::from_raw(0x0102)
            );
            assert_eq!(
                ChessBitboard::anti_diag_for_sq(ChessSquare::from_bb_index(15), ChessboardSize::default()),
                ChessBitboard::from_raw(0x0204_0810_2040_8000)
            );
            assert_eq!(
                ChessBitboard::anti_diag_for_sq(ChessSquare::from_bb_index(42), ChessboardSize::default()),
                ChessBitboard::anti_diag_for_sq(ChessSquare::from_bb_index(35), ChessboardSize::default())
            );
        }

        #[test]
        fn flip_left_right_test() {
            assert_eq!(ChessBitboard::from_raw(0).flip_lowest_row(), ChessBitboard::from_raw(0));
            assert_eq!(ChessBitboard::from_raw(1).flip_lowest_row(), ChessBitboard::from_raw(0x80));
            assert_eq!(
                ChessBitboard::from_raw(0x0003_4010_00e0).flip_lowest_row().raw() & 0xff,
                ChessBitboard::from_raw(0x00c0_0208_0007).raw() & 0xff
            );
            assert_eq!(
                ChessBitboard::from_raw(0xffff_ffff_ffff_fffe).flip_lowest_row(),
                ChessBitboard::from_raw(0xffff_ffff_ffff_ff7f & 0xff)
            );
        }

        #[test]
        fn flip_up_down_test() {
            assert_eq!(ChessBitboard::from_raw(0).flip_up_down(), ChessBitboard::from_raw(0));
            assert_eq!(ChessBitboard::from_raw(1).flip_up_down(), ChessBitboard::from_raw(0x0100_0000_0000_0000));
            assert_eq!(
                ChessBitboard::from_raw(0x0340_1000_e000_00ac).flip_up_down(),
                ChessBitboard::from_raw(0xac00_00e0_0010_4003)
            );
            assert_eq!(
                ChessBitboard::from_raw(0xffff_ffff_ffff_fffe).flip_up_down(),
                ChessBitboard::from_raw(0xfeff_ffff_ffff_ffff)
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
            assert!(!(0x0 as ExtendedRawBitboard).is_single_piece());
            assert!((0x1 as ExtendedRawBitboard).is_single_piece());
            assert!((0x2 as ExtendedRawBitboard).is_single_piece());
            assert!(!(0x3 as ExtendedRawBitboard).is_single_piece());
            assert!((0x4 as ExtendedRawBitboard).is_single_piece());
            assert!((0x400 as ExtendedRawBitboard).is_single_piece());
            assert!(!(0x4001 as ExtendedRawBitboard).is_single_piece());
            assert!((LARGER_THAN_64_BIT as ExtendedRawBitboard).is_single_piece());
            assert!((0x200_0000_0000_0000_0000_0000_0000_0000 as ExtendedRawBitboard).is_single_piece());
            assert!(!(!(0x0 as ExtendedRawBitboard)).is_single_piece());
        }

        #[test]
        fn trailing_zeros_test() {
            assert_eq!((0 as ExtendedRawBitboard).num_trailing_zeros(), 128);
            assert_eq!((1 as ExtendedRawBitboard).num_trailing_zeros(), 0);
            assert_eq!((2 as ExtendedRawBitboard).num_trailing_zeros(), 1);
            assert_eq!((0xa as ExtendedRawBitboard).num_trailing_zeros(), 1);
            assert_eq!((0xa0bc_00de_f000 as ExtendedRawBitboard).num_trailing_zeros(), 12);
            assert_eq!(((0xa0bc_00de_f000 + LARGER_THAN_64_BIT) as ExtendedRawBitboard).num_trailing_zeros(), 12);
            assert_eq!(((!0) as ExtendedRawBitboard).num_trailing_zeros(), 0);
            assert_eq!(((!0 << 2) as ExtendedRawBitboard).num_trailing_zeros(), 2);
        }

        #[test]
        fn diag_test() {
            let size = GridSize::new(Height(1), Width(2));
            assert_eq!(
                MnkBitboard::diag_for_sq(GridCoordinates::from_rank_file(0, 0), size,) & MnkBitboard::new(0b11, size),
                MnkBitboard::new(1, size)
            );
            let size = GridSize::new(Height(3), Width(2));
            assert_eq!(
                MnkBitboard::diag_for_sq(GridCoordinates::from_rank_file(0, 0), size,),
                MnkBitboard::new(0b00_1001, size)
            );
            assert_eq!(
                MnkBitboard::diag_for_sq(GridCoordinates::from_rank_file(0, 1), size,),
                MnkBitboard::new(0b10, size)
            );
            let size = GridSize::new(Height(11), Width(8));
            assert_eq!(
                MnkBitboard::diag_for_sq(GridCoordinates::from_rank_file(0, 7), size,),
                MnkBitboard::new(0x80, size)
            );
            let size = GridSize::new(Height(9), Width(8));
            assert_eq!(
                MnkBitboard::diag_for_sq(GridCoordinates::from_rank_file(1, 0), size,)
                    & MnkBitboard::new(u64::MAX as u128, size),
                MnkBitboard::new(0x4020_1008_0402_0100, size)
            );
            let size = GridSize::new(Height(7), Width(3));
            assert_eq!(
                MnkBitboard::diag_for_sq(GridCoordinates::from_rank_file(0, 1), size,),
                MnkBitboard::new(0b100_010, size)
            );
            for width in 1..12 {
                for square in width + 1..9 * width {
                    let prev = square - width - 1;
                    if square % width == 0 {
                        continue;
                    }
                    let size = GridSize::new(Height(9), Width(width));
                    assert_eq!(
                        MnkBitboard::diag_for_sq(GridCoordinates::from_rank_file(square / width, square % width), size,),
                        MnkBitboard::diag_for_sq(GridCoordinates::from_rank_file(prev / width, prev % width), size)
                    );
                }
            }
        }

        #[test]
        fn anti_diag_test() {
            let size = GridSize::new(Height(3), Width(1));
            assert_eq!(
                MnkBitboard::anti_diag_for_sq(GridCoordinates::from_rank_file(0, 0), size,),
                MnkBitboard::new(1, size)
            );
            let size = GridSize::new(Height(4), Width(1));
            assert_eq!(
                MnkBitboard::anti_diag_for_sq(GridCoordinates::from_rank_file(1, 0), size,),
                MnkBitboard::new(0b10, size)
            );
            let size = GridSize::new(Height(3), Width(2));
            assert_eq!(
                MnkBitboard::anti_diag_for_sq(GridCoordinates::from_rank_file(0, 1), size,)
                    & MnkBitboard::new(0b111_111, size),
                MnkBitboard::new(0b0110, size)
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
                            GridCoordinates::from_rank_file(square / width, square % width),
                            size,
                        ),
                        MnkBitboard::anti_diag_for_sq(
                            GridCoordinates::from_rank_file(prev / width, prev % width),
                            size,
                        )
                    );
                }
            }
        }

        #[test]
        fn flip_left_right_test() {
            let size = GridSize::new(Height(4), Width(3));
            assert_eq!(MnkBitboard::new(0, size).flip_lowest_row(), MnkBitboard::new(0, size));
            let size = GridSize::chess();
            assert_eq!(MnkBitboard::new(1, size).flip_lowest_row(), MnkBitboard::new(0b1000_0000, size));
            let size = GridSize::new(Height(12), Width(2));
            assert_eq!(MnkBitboard::new(0x02_34e1, size).flip_lowest_row(), MnkBitboard::new(0x01_38d2, size));
            let size = GridSize::new(Height(7), Width(3));
            assert_eq!(
                MnkBitboard::new(0b101_001_011_110_111_001, size).flip_lowest_row(),
                MnkBitboard::new(0b101_100_110_011_111_100, size)
            );
            let size = GridSize::new(Height(50), Width(3));
            assert_eq!(
                MnkBitboard::new(0x4924_9249_2492_4924_9249_2492_4924_9249, size).flip_lowest_row(),
                MnkBitboard::new(0x2492_4924_9249_2492_4924_9249_2492_4924, size)
            );
        }

        #[test]
        fn flip_up_down_test() {
            let size = GridSize::new(Height(7), Width(3));
            assert_eq!(MnkBitboard::new(0, size).flip_up_down(), MnkBitboard::new(0, size));
            let size = GridSize::new(Height(2), Width(9));
            assert_eq!(MnkBitboard::new(1, size).flip_up_down(), MnkBitboard::new(0x200, size));
            let size = GridSize::new(Height(12), Width(8));
            assert_eq!(
                MnkBitboard::new(0x0340_1000_e000_00ac, size).flip_up_down(),
                MnkBitboard::new(0xac00_00e0_0010_4003_0000_0000, size)
            );
            let size = GridSize::new(Height(3), Width(10));
            assert_eq!(
                MnkBitboard::new(0b00110_01010_11001_11010, size).flip_up_down(),
                MnkBitboard::new(0b11001_11010_00110_01010_00000_00000, size)
            );
            let size = GridSize::tictactoe();
            assert_eq!(MnkBitboard::new(0b001_001_001, size).flip_up_down(), MnkBitboard::new(0b001_001_001, size));
            assert_eq!(
                MnkBitboard::new(0x4924_9249_2492_4924_9249_2492_4924_9249, size).flip_up_down(),
                MnkBitboard::new(0x4924_9249_2492_4924_9249_2492_4924_9249, size)
            );
        }
    }
}
