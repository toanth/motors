use std::time::Duration;

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use gears::games::chess::ChessColor::White;
use gears::games::chess::Chessboard;
use gears::general::bitboards::{Bitboard, RawBitboard};
use gears::general::board::Strictness::Relaxed;
use gears::general::board::{BitboardBoard, Board, BoardHelpers};
use gears::general::perft::perft;
use gears::search::DepthPly;

const QUEENS_FEN: &str = "k7/3Q3Q/8/2Q5/2Q3Q1/2Q5/2QQ3Q/KQ6 w - - 0 1";
const ROOKS_FEN: &str = "k7/4R3/5R2/8/2R3R1/2R5/2RR3R/KRR5 w - - 0 1";
const BISHOPS_FEN: &str = "k7/3B3B/8/8/2B3B1/2BB4/2BB3B/KB6 w - - 0 1";
const KNIGHTS_FEN: &str = "k6N/3N4/8/2NN4/2N1N1N1/2N5/2NN4/K7 w - - 0 1";
const PAWNS_FEN: &str = "k7/3P3P/7p/1p3pP1/2P5/3Pp3/2PP3P/K7 w - f6 0 2";

pub fn perft_startpos_bench(c: &mut Criterion) {
    c.bench_function("perft 4 startpos", |b| {
        let pos = Chessboard::default();
        b.iter(|| perft(DepthPly::new(4), pos, false));
    });
}

pub fn perft_kiwipete_bench(c: &mut Criterion) {
    c.bench_function("perft 4 kiwipete", |b| {
        let pos = Chessboard::from_name("kiwipete").unwrap();
        b.iter(|| perft(DepthPly::new(4), pos, false));
    });
}

fn gen_moves(c: &mut Criterion, name: &str, fen: &str) {
    c.bench_function(name, |b| {
        let pos = Chessboard::from_fen(fen, Relaxed).unwrap();
        b.iter(|| black_box(pos).pseudolegal_moves());
    });
}

fn play_moves(c: &mut Criterion, name: &str, fen: &str) {
    c.bench_function(name, |b| {
        let pos = Chessboard::from_fen(fen, Relaxed).unwrap();
        let moves = pos.pseudolegal_moves();
        b.iter(|| {
            for m in &moves {
                black_box(black_box(pos).make_move(*m));
            }
        });
    });
}

pub fn gen_knight_moves_bench(c: &mut Criterion) {
    gen_moves(c, "gen knight moves", KNIGHTS_FEN);
}

pub fn gen_queen_moves_bench(c: &mut Criterion) {
    gen_moves(c, "gen queen moves", QUEENS_FEN);
}

pub fn gen_rook_moves_bench(c: &mut Criterion) {
    gen_moves(c, "gen rook moves", ROOKS_FEN);
}

pub fn gen_bishop_moves_bench(c: &mut Criterion) {
    gen_moves(c, "gen bishop moves", BISHOPS_FEN);
}

pub fn gen_pawn_moves_bench(c: &mut Criterion) {
    gen_moves(c, "gen pawn moves", PAWNS_FEN);
}

pub fn play_knight_moves(c: &mut Criterion) {
    play_moves(c, "play knight moves", KNIGHTS_FEN);
}

pub fn play_queen_moves(c: &mut Criterion) {
    play_moves(c, "play queen moves", QUEENS_FEN);
}

pub fn play_rook_moves(c: &mut Criterion) {
    play_moves(c, "play rook moves", ROOKS_FEN);
}

pub fn play_bishop_moves(c: &mut Criterion) {
    play_moves(c, "play bishop moves", BISHOPS_FEN);
}

pub fn play_pawn_moves(c: &mut Criterion) {
    play_moves(c, "play bishop moves", PAWNS_FEN);
}

pub fn bitboard_ones_bench(c: &mut Criterion) {
    c.bench_function("bitboard ones", |b| {
        let positions = Chessboard::bench_positions();
        b.iter(|| {
            for pos in &positions {
                let mut sum = 0;
                for piece in pos.player_bb(White).ones() {
                    sum += piece.bb_idx();
                }
                black_box(sum);
            }
        });
    });
}

pub fn bitboard_poplsb_bench(c: &mut Criterion) {
    c.bench_function("bitboard poplsb", |b| {
        let positions = Chessboard::bench_positions();
        b.iter(|| {
            let mut sum = 0;
            for pos in &positions {
                let mut bb = pos.player_bb(White);
                while bb.has_set_bit() {
                    sum += bb.pop_lsb();
                }
                black_box(sum);
            }
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(20)).noise_threshold(0.03);
    targets =
    perft_startpos_bench,
    perft_kiwipete_bench,
    gen_pawn_moves_bench,
    gen_knight_moves_bench,
    gen_bishop_moves_bench,
    gen_rook_moves_bench,
    gen_queen_moves_bench,
    play_pawn_moves,
    play_knight_moves,
    play_bishop_moves,
    play_rook_moves,
    play_queen_moves,
    bitboard_ones_bench,
    bitboard_poplsb_bench,
}

criterion_main!(benches);
