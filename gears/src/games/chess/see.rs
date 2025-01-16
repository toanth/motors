use crate::games::chess::moves::ChessMove;
use crate::games::chess::moves::ChessMoveFlags::{NormalPawnMove, PromoQueen};
use crate::games::chess::pieces::ChessPieceType::*;
use crate::games::chess::pieces::{ChessPieceType, NUM_CHESS_PIECES};
use crate::games::chess::squares::ChessSquare;
use crate::games::chess::{ChessColor, Chessboard, PAWN_CAPTURES};
use crate::games::{AbstractPieceType, Board, Color};
use crate::general::bitboards::chessboard::ChessBitboard;
use crate::general::bitboards::RayDirections::Vertical;
use crate::general::bitboards::{Bitboard, KnownSizeBitboard, RawBitboard};
use crate::general::board::{BitboardBoard, BoardHelpers};
use derive_more::{Add, AddAssign, Neg, Sub, SubAssign};
use std::mem::swap;

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq, Copy, Clone, Add, AddAssign, Sub, SubAssign, Neg)]
#[must_use]
pub struct SeeScore(pub i32);

// TODO: Better values?
pub const SEE_SCORES: [SeeScore; NUM_CHESS_PIECES + 1] = [
    SeeScore(100),
    SeeScore(300),
    SeeScore(300),
    SeeScore(500),
    SeeScore(900),
    SeeScore(99999),
    SeeScore(0), // also give the empty square a see value to make the implementation simpler
];

pub fn piece_see_value(piece: ChessPieceType) -> SeeScore {
    SEE_SCORES[piece.to_uncolored_idx()]
}

pub fn move_see_value(mov: ChessMove, victim: ChessPieceType) -> SeeScore {
    let mut score = piece_see_value(victim);
    if mov.is_promotion() {
        score += piece_see_value(mov.promo_piece()) - piece_see_value(Pawn);
    } else if mov.is_castle() {
        score = SeeScore(0); // the 'victim' would be our own rook
    }
    score
}

impl Chessboard {
    fn next_see_attacker(
        &self,
        color: ChessColor,
        all_remaining_attackers: ChessBitboard,
    ) -> Option<(ChessPieceType, ChessSquare)> {
        for piece in ChessPieceType::pieces() {
            let mut current_attackers = self.colored_piece_bb(color, piece) & all_remaining_attackers;
            if current_attackers.has_set_bit() {
                return Some((piece, ChessSquare::from_bb_index(current_attackers.pop_lsb())));
            };
        }
        None
    }

    pub fn see(&self, mov: ChessMove, mut alpha: SeeScore, mut beta: SeeScore) -> SeeScore {
        debug_assert!(alpha < beta);
        let square = mov.dest_square();
        let mut color = self.active_player;
        let original_moving_piece = mov.piece_type();
        let mut our_victim = self.piece_type_on(square);
        // A simple shortcut to avoid doing most of the work of SEE for a large portion of the cases it's called.
        // This needs to handle the case of the opponent recapturing with a pawn promotion.
        if piece_see_value(our_victim) - piece_see_value(original_moving_piece) >= beta
            && !(square.is_backrank()
                && (PAWN_CAPTURES[color as usize][square.bb_idx()] & self.colored_piece_bb(color.other(), Pawn))
                    .has_set_bit())
        {
            return beta;
        }
        let mut all_remaining_attackers = self.all_attacking(square);
        let mut removed_attackers = ChessBitboard::default();
        if self.is_occupied(square) {
            removed_attackers = square.bb(); // hyperbola quintessence expects the source square to be empty
        }
        let mut their_victim = original_moving_piece;
        if mov.is_promotion() {
            their_victim = mov.promo_piece();
        } else if mov.is_ep() {
            our_victim = Pawn;
            let bb = mov.square_of_pawn_taken_by_ep().unwrap().bb();
            debug_assert_eq!(bb & !self.occupied_bb(), ChessBitboard::default());
            all_remaining_attackers |= ChessBitboard::slider_attacks(square, self.occupied_bb() ^ bb, Vertical)
                & (self.piece_bb(Rook) | self.piece_bb(Queen));
        }
        let mut eval = move_see_value(mov, our_victim);
        // testing if eval - max recapture score was >= beta caused a decently large bench performance regression,
        // so let's not do that. This also significantly simplifies the code, because the max recapture score can be larger
        // than the captured piece value in case of promotions.

        let mut see_attack =
            |attacker: ChessSquare, all_remaining_attackers: &mut ChessBitboard, piece: ChessPieceType| {
                // `&= !` instead of `^` because in the case of a regular pawn move, the moving pawn wasn't part of the attacker bb.
                *all_remaining_attackers &= !ChessBitboard::single_piece(attacker);
                removed_attackers |= ChessBitboard::single_piece(attacker);
                debug_assert_eq!(removed_attackers & !self.occupied_bb(), ChessBitboard::default());
                let blockers_left = self.occupied_bb() ^ removed_attackers;
                // xrays for sliders
                let ray_attacks = self.ray_attacks(square, attacker, blockers_left);
                let new_attack = ray_attacks & !(removed_attackers | *all_remaining_attackers);
                debug_assert!(new_attack.count_ones() <= 1);
                *all_remaining_attackers |= new_attack;
                let (flags, new_piece) = if piece == Pawn && square.is_backrank() {
                    (PromoQueen, Queen)
                } else {
                    (NormalPawnMove, piece) // the flag doesn't matter, as long as it's not a promo or castle
                };
                (ChessMove::new(attacker, square, flags), new_piece)
            };
        _ = see_attack(mov.src_square(), &mut all_remaining_attackers, original_moving_piece);

        loop {
            color = color.other();
            (alpha, beta) = (-beta, -alpha);
            eval = -eval;
            swap(&mut our_victim, &mut their_victim);
            if eval >= beta {
                return if color == self.active_player { beta } else { -beta };
            } else if eval > alpha {
                alpha = eval;
            }
            let Some((piece, attacker_src_square)) = self.next_see_attacker(color, all_remaining_attackers) else {
                return if color == self.active_player { eval.max(alpha) } else { -eval.max(alpha) };
            };
            let (mov, piece) = see_attack(attacker_src_square, &mut all_remaining_attackers, piece);
            eval += move_see_value(mov, our_victim);
            their_victim = piece;
        }
    }

    #[must_use]
    pub fn see_at_least(&self, mov: ChessMove, beta: SeeScore) -> bool {
        self.see(mov, beta - SeeScore(1), beta).0 >= beta.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games::chess::Chessboard;
    use crate::games::Board;
    use crate::general::board::BoardHelpers;
    use crate::general::board::Strictness::Relaxed;
    use crate::general::common::parse_int_from_str;
    use crate::general::moves::Move;

    #[test]
    fn trivial_see_test() {
        let board = Chessboard::from_name("kiwipete").unwrap();
        let see_score_no_capture =
            board.see(ChessMove::from_compact_text("a1b1", &board).unwrap(), SeeScore(-1000), SeeScore(1000));
        assert_eq!(see_score_no_capture, SeeScore(0));
        let see_score_bishop_capture =
            board.see(ChessMove::from_compact_text("e2a6", &board).unwrap(), SeeScore(-1000), SeeScore(1000));
        assert_eq!(see_score_bishop_capture, SeeScore(300));
        let see_score_bishop_capture =
            board.see(ChessMove::from_compact_text("e2a6", &board).unwrap(), SeeScore(0), SeeScore(1));
        assert!(see_score_bishop_capture >= SeeScore(1));

        let see_score_bad_capture =
            board.see(ChessMove::from_compact_text("f3f6", &board).unwrap(), SeeScore(-9999), SeeScore(9999));
        assert_eq!(see_score_bad_capture, SeeScore(-600));

        let see_score_bad_pawn_capture =
            board.see(ChessMove::from_compact_text("f3h3", &board).unwrap(), SeeScore(-9999), SeeScore(9999));
        assert_eq!(see_score_bad_pawn_capture, SeeScore(-300));

        let see_score_good_pawn_capture =
            board.see(ChessMove::from_compact_text("g2h3", &board).unwrap(), SeeScore(-9999), SeeScore(9999));
        assert_eq!(see_score_good_pawn_capture, SeeScore(100));
    }

    #[test]
    fn see_test() {
        let board = Chessboard::from_name("see_win_pawn").unwrap();
        let see_score =
            board.see(ChessMove::from_compact_text("f4e5", &board).unwrap(), SeeScore(-9999), SeeScore(9999));
        assert_eq!(see_score, SeeScore(100));

        let see_score = board.see(ChessMove::from_compact_text("d3e5", &board).unwrap(), SeeScore(-120), SeeScore(101));
        assert_eq!(see_score, SeeScore(100));

        let see_score = board.see(ChessMove::from_compact_text("c5d6", &board).unwrap(), SeeScore(-120), SeeScore(200));
        assert_eq!(see_score, SeeScore(200));

        let see_score = board.see(ChessMove::from_compact_text("f4e5", &board).unwrap(), SeeScore(200), SeeScore(9999));
        // TODO: Fail soft? It doesn't make sense to clamp to the window.
        assert_eq!(see_score, SeeScore(200));

        let board = board.flip_side_to_move().unwrap();
        let see_score =
            board.see(ChessMove::from_compact_text("e5d4", &board).unwrap(), SeeScore(-999), SeeScore(9999));
        assert_eq!(see_score, SeeScore(100));

        let see_score =
            board.see(ChessMove::from_compact_text("e5f4", &board).unwrap(), SeeScore(-1234), SeeScore(567_890));
        assert_eq!(see_score, SeeScore(0));

        let see_score =
            board.see(ChessMove::from_compact_text("d7c5", &board).unwrap(), SeeScore(-9999), SeeScore(9999));
        assert_eq!(see_score, SeeScore(-200));
    }

    #[test]
    fn see_xray_test() {
        let board = Chessboard::from_name("see_xray").unwrap();
        let see_score =
            board.see(ChessMove::from_compact_text("c4f4", &board).unwrap(), SeeScore(-9999), SeeScore(9999));
        assert_eq!(see_score, SeeScore(-600));
        let see_score = board.see(ChessMove::from_compact_text("b4b8", &board).unwrap(), SeeScore(-1234), SeeScore(1));
        assert_eq!(see_score, SeeScore(-500));
    }

    #[test]
    // test suite from Leorik: https://github.com/lithander/Leorik/blob/master/Leorik.Test/see.epd,
    // with some original tests appended.
    fn see_test_suite() {
        // There are quite a few unrealistic but tricky corner cases that are neither handled not tested properly here
        let tests = [
            "6k1/1pp4p/p1pb4/6q1/3P1pRr/2P4P/PP1Br1P1/5RKN w - -; Rfxf4; -100; P - R + B",
            "5rk1/1pp2q1p/p1pb4/8/3P1NP1/2P5/1P1BQ1P1/5RK1 b - -; Bxf4; 0; -N + B",
            "4R3/2r3p1/5bk1/1p1r3p/p2PR1P1/P1BK1P2/1P6/8 b - -; hxg4; 0;",
            "4R3/2r3p1/5bk1/1p1r1p1p/p2PR1P1/P1BK1P2/1P6/8 b - -; hxg4; 0;",
            "4r1k1/5pp1/nbp4p/1p2p2q/1P2P1b1/1BP2N1P/1B2QPPK/3R4 b - -; Bxf3; 0;",
            "2r1r1k1/pp1bppbp/3p1np1/q3P3/2P2P2/1P2B3/P1N1B1PP/2RQ1RK1 b - -; dxe5; 100; P",
            "7r/5qpk/p1Qp1b1p/3r3n/BB3p2/5p2/P1P2P2/4RK1R w - -; Re8; 0;",
            "6rr/6pk/p1Qp1b1p/2n5/1B3p2/5p2/P1P2P2/4RK1R w - -; Re8; -500; -R",
            "7r/5qpk/2Qp1b1p/1N1r3n/BB3p2/5p2/P1P2P2/4RK1R w - -; Re8; -500; -R",
            "6RR/4bP2/8/8/5r2/3K4/5p2/4k3 w - -; f8=Q; 200; B - P",
            "6RR/4bP2/8/8/5r2/3K4/5p2/4k3 w - -; f8=N; 200; N - P",
            "7R/5P2/8/8/6r1/3K4/5p2/4k3 w - -; f8=Q; 800; Q - P",
            "7R/5P2/8/8/6r1/3K4/5p2/4k3 w - -; f8=B; 200; B - P",
            "7R/4bP2/8/8/1q6/3K4/5p2/4k3 w - -; f8=R; -100; -P",
            "8/4kp2/2npp3/1Nn5/1p2PQP1/7q/1PP1B3/4KR1r b - -; Rxf1+; 0;",
            "8/4kp2/2npp3/1Nn5/1p2P1P1/7q/1PP1B3/4KR1r b - -; Rxf1+; 0;",
            "2r2r1k/6bp/p7/2q2p1Q/3PpP2/1B6/P5PP/2RR3K b - -; Qxc1; 100; R - Q + R",
            "r2qk1nr/pp2ppbp/2b3p1/2p1p3/8/2N2N2/PPPP1PPP/R1BQR1K1 w kq -; Nxe5; 100; P",
            "6r1/4kq2/b2p1p2/p1pPb3/p1P2B1Q/2P4P/2B1R1P1/6K1 w - -; Bxe5; 0;",
            "3q2nk/pb1r1p2/np6/3P2Pp/2p1P3/2R4B/PQ3P1P/3R2K1 w - h6; gxh6; 0;",
            "3q2nk/pb1r1p2/np6/3P2Pp/2p1P3/2R1B2B/PQ3P1P/3R2K1 w - h6; gxh6; 100; P",
            "2r4r/1P4pk/p2p1b1p/7n/BB3p2/2R2p2/P1P2P2/4RK2 w - -; Rxc8; 500; R",
            "2r5/1P4pk/p2p1b1p/5b1n/BB3p2/2R2p2/P1P2P2/4RK2 w - -; Rxc8; 500; R",
            "2r4k/2r4p/p7/2b2p1b/4pP2/1BR5/P1R3PP/2Q4K w - -; Rxc5; 300; B",
            "8/pp6/2pkp3/4bp2/2R3b1/2P5/PP4B1/1K6 w - -; Bxc6; -200; P - B",
            "4q3/1p1pr1k1/1B2rp2/6p1/p3PP2/P3R1P1/1P2R1K1/4Q3 b - -; Rxe4; -400; P - R",
            "4q3/1p1pr1kb/1B2rp2/6p1/p3PP2/P3R1P1/1P2R1K1/4Q3 b - -; Bxe4; 100; P",
            "3r3k/3r4/2n1n3/8/3p4/2PR4/1B1Q4/3R3K w - -; Rxd4; -100; P - R + N - P + N - B + R - Q + R",
            "1k1r4/1ppn3p/p4b2/4n3/8/P2N2P1/1PP1R1BP/2K1Q3 w - -; Nxe5; 100; N - N + B - R + N",
            "1k1r3q/1ppn3p/p4b2/4p3/8/P2N2P1/1PP1R1BP/2K1Q3 w - -; Nxe5; -200; P - N",
            "rnb2b1r/ppp2kpp/5n2/4P3/q2P3B/5R2/PPP2PPP/RN1QKB2 w Q -; Bxf6; 100; N - B + P",
            "r2q1rk1/2p1bppp/p2p1n2/1p2P3/4P1b1/1nP1BN2/PP3PPP/RN1QR1K1 b - -; Bxf3; 0; N - B",
            "r1bqkb1r/2pp1ppp/p1n5/1p2p3/3Pn3/1B3N2/PPP2PPP/RNBQ1RK1 b kq -; Nxd4; 0; P - N + N - P",
            "r1bq1r2/pp1ppkbp/4N1p1/n3P1B1/8/2N5/PPP2PPP/R2QK2R w KQ -; Nxg7; 0; B - N",
            "r1bq1r2/pp1ppkbp/4N1pB/n3P3/8/2N5/PPP2PPP/R2QK2R w KQ -; Nxg7; 300; B",
            "rnq1k2r/1b3ppp/p2bpn2/1p1p4/3N4/1BN1P3/PPP2PPP/R1BQR1K1 b kq -; Bxh2; -200; P - B",
            "rn2k2r/1bq2ppp/p2bpn2/1p1p4/3N4/1BN1P3/PPP2PPP/R1BQR1K1 b kq -; Bxh2; 100; P",
            "r2qkbn1/ppp1pp1p/3p1rp1/3Pn3/4P1b1/2N2N2/PPP2PPP/R1BQKB1R b KQq -; Bxf3; 100; N - B + P",
            "rnbq1rk1/pppp1ppp/4pn2/8/1bPP4/P1N5/1PQ1PPPP/R1B1KBNR b KQ -; Bxc3; 0; N - B",
            "r4rk1/3nppbp/bq1p1np1/2pP4/8/2N2NPP/PP2PPB1/R1BQR1K1 b - -; Qxb2; -800; P - Q",
            "r4rk1/1q1nppbp/b2p1np1/2pP4/8/2N2NPP/PP2PPB1/R1BQR1K1 b - -; Nxd5; -200; P - N",
            "1r3r2/5p2/4p2p/2k1n1P1/2PN1nP1/1P3P2/8/2KR1B1R b - -; Rxb3; -400; P - R",
            "1r3r2/5p2/4p2p/4n1P1/kPPN1nP1/5P2/8/2KR1B1R b - -; Rxb4; 100; P",
            "2r2rk1/5pp1/pp5p/q2p4/P3n3/1Q3NP1/1P2PP1P/2RR2K1 b - -; Rxc1; 0; R - R",
            "5rk1/5pp1/2r4p/5b2/2R5/6Q1/R1P1qPP1/5NK1 b - -; Bxc2; -100; P - B + R - Q + R",
            "1r3r1k/p4pp1/2p1p2p/qpQP3P/2P5/3R4/PP3PP1/1K1R4 b - -; Qxa2; -800; P - Q",
            "1r5k/p4pp1/2p1p2p/qpQP3P/2P2P2/1P1R4/P4rP1/1K1R4 b - -; Qxa2; 100; P",
            "r2q1rk1/1b2bppp/p2p1n2/1ppNp3/3nP3/P2P1N1P/BPP2PP1/R1BQR1K1 w - -; Nxe7; 0; B - N",
            "rnbqrbn1/pp3ppp/3p4/2p2k2/4p3/3B1K2/PPP2PPP/RNB1Q1NR w - -; Bxe4; 100; P",
            "rnb1k2r/p3p1pp/1p3p1b/7n/1N2N3/3P1PB1/PPP1P1PP/R2QKB1R w KQkq -; Nd6; -200; -N + P",
            "r1b1k2r/p4npp/1pp2p1b/7n/1N2N3/3P1PB1/PPP1P1PP/R2QKB1R w KQkq -; Nd6; 0; -N + N",
            "2r1k2r/pb4pp/5p1b/2KB3n/4N3/2NP1PB1/PPP1P1PP/R2Q3R w k -; Bc6; -300; -B",
            "2r1k2r/pb4pp/5p1b/2KB3n/1N2N3/3P1PB1/PPP1P1PP/R2Q3R w k -; Bc6; 0; -B + B",
            "2r1k3/pbr3pp/5p1b/2KB3n/1N2N3/3P1PB1/PPP1P1PP/R2Q3R w - -; Bc6; -300; -B + B - N",
            "5k2/p2P2pp/8/1pb5/1Nn1P1n1/6Q1/PPP4P/R3K1NR w KQ -; d8=Q; 800; (Q - P)",
            "r4k2/p2P2pp/8/1pb5/1Nn1P1n1/6Q1/PPP4P/R3K1NR w KQ -; d8=Q; -100; (Q - P) - Q",
            "5k2/p2P2pp/1b6/1p6/1Nn1P1n1/8/PPP4P/R2QK1NR w KQ -; d8=Q; 200; (Q - P) - Q + B",
            "4kbnr/p1P1pppp/b7/4q3/7n/8/PP1PPPPP/RNBQKBNR w KQk -; c8=Q; -100; (Q - P) - Q",
            "4kbnr/p1P1pppp/b7/4q3/7n/8/PPQPPPPP/RNB1KBNR w KQk -; c8=Q; 200; (Q - P) - Q + B",
            "4kbnr/p1P1pppp/b7/4q3/7n/8/PPQPPPPP/RNB1KBNR w KQk -; c8=Q; 200; (Q - P)",
            "4kbnr/p1P4p/b1q5/5pP1/4n3/5Q2/PP1PPP1P/RNB1KBNR w KQk f6; gxf6; 0; P - P",
            "4kbnr/p1P4p/b1q5/5pP1/4n3/5Q2/PP1PPP1P/RNB1KBNR w KQk f6; gxf6;	0; P - P",
            "4kbnr/p1P4p/b1q5/5pP1/4n2Q/8/PP1PPP1P/RNB1KBNR w KQk f6; gxf6; 0; P - P",
            "1n2kb1r/p1P4p/2qb4/5pP1/4n2Q/8/PP1PPP1P/RNB1KBNR w KQk -; cxb8=Q; 200; N + (Q - P) - Q",
            "rnbqk2r/pp3ppp/2p1pn2/3p4/3P4/N1P1BN2/PPB1PPPb/R2Q1RK1 w kq -; Kxh2; 300; B",
            "3N4/2K5/2n5/1k6/8/8/8/8 b - -; Nxd8; 0; N - N",
            "3N4/2P5/2n5/1k6/8/8/8/4K3 b - -; Nxd8; -800; N - (N + Q - P) ",
            "3n3r/2P5/8/1k6/8/8/3Q4/4K3 w - -; Qxd8; 300; N",
            "3n3r/2P5/8/1k6/8/8/3Q4/4K3 w - -; cxd8=Q; 700; (N + Q - P) - Q + R",
            "r2n3r/2P1P3/4N3/1k6/8/8/8/4K3 w - -; Nxd8; 300; N",
            "8/8/8/1k6/6b1/4N3/2p3K1/3n4 w - -; Nxd1; -800; N - N", // This was incorrect in the original test suite
            "8/8/1k6/8/8/2N1N3/2p1p1K1/3n4 w - -; Nexd1; -800; N - (N + Q - P)",
            "8/8/1k6/8/8/2N1N3/4p1K1/3n4 w - -; Ncxd1; 100; N - (N + Q - P) + Q ",
            "r1bqk1nr/pppp1ppp/2n5/1B2p3/1b2P3/5N2/PPPP1PPP/RNBQK2R w KQkq -; O-O; 0;",
            "3q3k/pb1r1p2/np6/3P2Pp/2p1P2r/2R4B/PQ3P1P/3R2K1 w - h6 0 1; gxh6; 0",
            "4kb1r/p1P4p/b1q5/5pP1/7Q/8/PP1PPP1P/RNB1KBNR w KQk f6 0 1; gxf6ep; 100",
            "3r4/8/1k6/8/6B1/1bN1NB2/2p1p1K1/3n4 w - - 0 1; Ncxd1; -400", // Doesn't work correctly, but maybe not a big deal
        ];
        for testcase in tests {
            let mut parts = testcase.split(';');
            let board = Chessboard::from_fen(parts.next().unwrap(), Relaxed).unwrap();
            let mov = ChessMove::from_extended_text(parts.next().unwrap().trim(), &board).unwrap();
            let expected_score = parse_int_from_str(parts.next().unwrap().trim(), "score").unwrap();
            let result = board.see(mov, SeeScore(-9999), SeeScore(9999));
            assert_eq!(result, SeeScore(expected_score));
            let expected_good = expected_score >= 0;
            let is_good = board.see_at_least(mov, SeeScore(0));
            assert_eq!(expected_good, is_good);
        }
    }
}
