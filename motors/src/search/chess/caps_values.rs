/*
 *  Motors, a collection of board game engines.
 *  Copyright (C) 2024 ToTheAnd
 *
 *  Motors is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  Motors is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with Motors. If not, see <https://www.gnu.org/licenses/>.
 */

use crate::spsa_params;

spsa_params! [
    soft_limit_divisor: u32 = 32; 5..=60; step=1;
    soft_limit_divisor_clamp: u32 = 4; 2..=10; step=1;
    hard_limit_divisor: usize = 4; 1..=10; step=1;
    move_stability_min_depth: isize=16; 1..=25; step=2;
    move_stability_start_divisor: usize = 4; 1..=10; step=1;
    move_stability_factor: usize = 750; 250..=1000; step=50;
    soft_limit_fail_low_factor: usize = 1250; 1000..=3000; step=50;
    aw_exact_add: ScoreT = 4; 0..=42; step=2;
    aw_exact_divisor: ScoreT = 2; 0..=10; step=1;
    aw_delta_max: u32 = 10; 0..=40; step=2;
    aw_widening_factor: ScoreT = 2; 1..=10; step=1;
    they_blundered_threshold: ScoreT = 50; 0..=200; step=5;
    we_blundered_threshold: ScoreT = -50; -200..=0; step=5;
    iir_min_depth: isize = 5; 1..=15; step=1;
    rfp_base: ScoreT = 150; 0..=900; step=15;
    rfp_blunder: ScoreT = 64; 0..=512; step=8;
    rfp_fail_high_divisor: ScoreT = 2; 1..=10; step=1;
    rfp_max_depth: isize = 3; 1..=10; step=1;
    nmp_fail_low: ScoreT = 64; 0..=256; step=4;
    nmp_min_depth: isize = 3; 1..=10; step=1;
    nmp_base: isize = 3; 1..=10; step=1;
    nmp_depth_div: isize = 4; 1..=20; step=1;
    nmp_verif_depth: isize = 8; 1..=20; step=1;
    fp_blunder_base: isize = 200; 0..=512; step=16;
    fp_blunder_scale: isize = 32; 1..=256; step=4;
    fp_base: isize = 300; 0..=800; step=16;
    fp_scale: isize = 64; 1..=512; step=4;
    lmp_blunder_base: isize = 6; 0..=32; step=1;
    lmp_blunder_scale: isize = 4; 0..=16; step=1;
    lmp_base: isize = 8; 0..=64; step=1;
    lmp_scale: isize = 8; 0..=32; step=1;
    lmp_fail_low_div: isize = 4; 2..=16; step=1;
    max_move_loop_pruning_depth: isize = 3; 1..=16; step=1;
    lmr_min_uninteresting: isize = 3; 0..=16; step=1;
    lmr_depth_div: isize = 8; 2..=16; step=1;
    lmr_const: isize = -1; -4..=8; step=1;
    lmr_bad_hist: i32 = -256; -800..=0; step=4;
    lmr_good_hist: i32 = 512; 0..=900; step=4;
    hist_depth_bonus: isize = 16; 1..=64; step=1;
];
