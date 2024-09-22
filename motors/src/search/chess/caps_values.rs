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
    soft_limit_div: u32 = 31; 5..=60; step=1;
    soft_limit_div_clamp: u32 = 3; 2..=10; step=1;
    hard_limit_div: usize = 3; 1..=10; step=1;
    move_stability_min_depth: isize=15; 1..=25; step=2;
    move_stability_start_div: usize = 4; 1..=10; step=1;
    move_stability_factor: usize = 785; 250..=1000; step=50;
    soft_limit_fail_low_factor: usize = 1215; 1000..=3000; step=50;
    aw_exact_add: ScoreT = 10; 0..=42; step=2;
    aw_exact_div: ScoreT = 3; 1..=10; step=1;
    aw_delta_max: u32 = 14; 0..=40; step=4;
    aw_widening_factor: ScoreT = 3; 1..=10; step=1;
    they_blundered_threshold: ScoreT = 59; 0..=200; step=5;
    we_blundered_threshold: ScoreT = -50; -200..=0; step=5;
    iir_min_depth: isize = 4; 1..=15; step=1;
    rfp_base: ScoreT = 153; 0..=900; step=15;
    rfp_blunder: ScoreT = 49; 0..=512; step=8;
    rfp_fail_high_div: ScoreT = 4; 1..=10; step=1;
    rfp_max_depth: isize = 4; 1..=10; step=1;
    nmp_fail_low: ScoreT = 59; 0..=256; step=4;
    nmp_min_depth: isize = 1; 1..=10; step=1;
    nmp_base: isize = 4; 1..=10; step=1;
    nmp_depth_div: isize = 4; 1..=20; step=1;
    nmp_verif_depth: isize = 7; 1..=20; step=1;
    fp_blunder_base: isize = 195; 0..=512; step=32;
    fp_blunder_scale: isize = 35; 1..=256; step=4;
    fp_base: isize = 284; 0..=800; step=32;
    fp_scale: isize = 59; 1..=512; step=4;
    lmp_blunder_base: isize = 2; 0..=32; step=1;
    lmp_blunder_scale: isize = 2; 0..=16; step=1;
    lmp_base: isize = 5; 0..=64; step=1;
    lmp_scale: isize = 7; 0..=32; step=1;
    lmp_fail_low_div: isize = 3; 2..=16; step=1;
    max_move_loop_pruning_depth: isize = 6; 1..=16; step=1;
    lmr_min_uninteresting: isize = 3; 0..=16; step=1;
    lmr_depth_div: isize = 7; 2..=16; step=1;
    lmr_const: isize = -1; -4..=8; step=1;
    lmr_bad_hist: i32 = -254; -800..=0; step=16;
    lmr_good_hist: i32 = 519; 0..=900; step=16  ;
    hist_depth_bonus: isize = 15; 1..=64; step=2;
];
