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

spsa_params![
    cc,
    start_depth: isize = 128; 0..=512; step=16;
    soft_limit_div: u32 = 30; 5..=60; step=1;
    inv_soft_limit_div_clamp: u32 = 256; 1..=750; step=10;
    inv_hard_limit_div: usize = 512; 1..=750; step=10;
    move_stability_min_iters: usize=14; 1..=25; step=2;
    move_stability_start_div: usize = 3; 1..=10; step=1;
    move_stability_factor: usize = 806; 250..=1000; step=50;
    soft_limit_fail_low_factor: usize = 1202; 1000..=3000; step=50;
    soft_limit_node_scale_min_depth: isize = 1024; 64..=2048; step=64;
    soft_limit_node_scale: u64 = 1400; 900..=2000; step = 50;
    fail_high_reduction_min_depth: isize = 8 * 128; 0..=2024; step=32;
    fail_high_reduction: isize = 128; 32..=512; step=32;
    fail_high_max_reduction: isize = 256; 64..=1024; step=32;
    aw_exact_add: ScoreT = 11; 0..=42; step=2;
    aw_exact_div: ScoreT = 3; 1..=10; step=1;
    aw_delta_max: u32 = 11; 0..=40; step=4;
    aw_widening_factor: ScoreT = 3; 1..=10; step=1;
    check_extension: isize = 128; 0..=256; step=16;
    low_depth_tt_extension_depth: isize = 6*128; 128..=10*128; step=32;
    tt_extension: isize = 128; 0..=256; step=16;
    they_blundered_threshold: ScoreT = 58; 0..=200; step=5;
    we_blundered_threshold: ScoreT = -47; -200..=0; step=5;
    iir_min_depth: isize = 512; 128..=32_768; step=32;
    iir_reduction: isize = 128; 0..=256; step=16;
    rfp_base: ScoreT = 158; 0..=900; step=15;
    rfp_blunder: ScoreT = 48; 0..=512; step=8;
    rfp_fail_high_div: ScoreT = 3; 1..=10; step=1;
    rfp_tt_upper_bound: ScoreT = 256; 0..=1024; step=64;
    rfp_noisy_pos: ScoreT = 1024; 0..=2048; step=128;
    rfp_max_depth: isize = 6 * 128; 128..=1024; step=32;
    razor_max_depth: isize = 256; 64..=1024; step=64;
    razor_depth_mult: isize = 4096; 256..32768; step=512;
    nmp_fail_low: ScoreT = 62; 0..=256; step=4;
    nmp_min_depth: isize = 128; 128..=1024; step=32;
    nmp_base: isize = 640; 256..=2048; step=32;
    nmp_depth_div: isize = 512; 128..=2048; step=64;
    nmp_blunder: isize = 128; 0..=512; step=16;
    nmp_verif_depth: isize = 1024; 128..=4096; step=32;
    fp_blunder_base: isize = 171 * 1024; 0..=512 * 1024; step=32 * 1024;
    fp_blunder_scale: isize = 37 * 8; 1..=256 * 8; step=16;
    fp_base: isize = 288 * 1024; 0..=800 * 1024; step=32 * 1024;
    fp_scale: isize = 58 * 8; 1..=512 * 8; step=16;
    lmp_blunder_base: isize = 2048; 0..=32 * 1024; step=512;
    lmp_blunder_scale: isize = 8; 0..=128; step=4;
    lmp_base: isize = 4096; 0..=64 * 1024; step=512;
    lmp_scale: isize = 40; 0..=128; step=4;
    lmp_fail_low_div: isize = 2; 2..=16; step=1;
    max_move_loop_pruning_depth: isize = 6 * 128; 1..=16_384; step=64;
    hist_pruning_max_depth: isize = 3 * 128; 128..=2048; step=32;
    max_see_pruning_depth: isize = 1024; 128..=2048; step=64;
    first_child_reduction: isize = 128; 64..=256; step=2;
    lmr_min_uninteresting: isize = 3; 0..=16; step=1;
    lmr_depth_div: isize = 8; 2..=16; step=1;
    lmr_moves_mult: isize = 128; 32..=256; step=4;
    lmr_const: isize = -128; -512..=1024; step=32;
    lmr_bad_hist: i16 = -257; -800..=0; step=16;
    lmr_bad_hist_reduction: isize = 128; 32..=256; step=8;
    lmr_good_hist: i16 = 525; 0..=900; step=16;
    lmr_good_hist_reduction: isize = 128; 32..=256; step=8;
    lmr_no_pv_reduction: isize = 128; 32..=256; step=8;
    lmr_we_blundered_reduction: isize = 128; 32..=256; step=8;
    lmr_new_in_check_reduction: isize = 128; 32..=256; step=8;
    lmr_in_check_reduction: isize = 128; 32..=256; step=8;
    min_fr_depth: isize = 7 * 128; 1..=8192; step=64;
    fr_base: ScoreT = 400; 100..=800; step=16;
    fr_scale: isize = 32 * 8; 64..=1024; step=8;
    fr_mult: isize = 128; 32..=256; step=4;
    tt_capt_reduction: isize = 128; 32..=256; step=8;
    lmr_bad_capthist: i16 = -500; -1000..=0; step=16;
    lmr_bad_capthist_reduction: isize = 128; 32..=256; step=8;
    lmr_good_capthist: i16 = 250; 0..=900; step=16;
    lmr_good_capthist_reduction: isize = 128; 32..=256; step=8;
    min_reduction_research: isize = 1; 0..=256; step=8;
    retry_base_reduction: isize = 128; 16..=256; step=16;
    do_deeper_base: ScoreT = 50; 8..=256; step=4;
    do_deeper_reduction: isize = 128; 32..=256; step=8;
    do_shallower_base: ScoreT = 10; 2..=128; step=4;
    do_shallower_reduction: isize = 128; 32..=256; step=8;
    third_search_reduction: isize = 128; 32..=256; step=2;
    hist_depth_bonus: isize = 128; 4..=512; step=32;
    hist_bonus_offset: isize = 0; -256..=256; step=8;
    hist_depth_malue: isize = 128; 4..=512; step=32;
    hist_malus_offset: isize = 0; -256..=256; step=8;
    main_hist_weight: isize = 1024; 256..=2048; step=16;
    countermove_weight: isize = 1024; 256..=2048; step=16;
    follow_up_weight: isize = 512; 256..=2048; step=16;
    corrhist_offset: isize = 128; 0..=512; step=32;
    corrhist_max: isize = 2048; 512..=65_536; step=256;
    nonpawn_corrhist_weight: isize = 512; 0..=2048; step=16;
    contcorrhist_weight: isize = 1024; 0..=2048; step=16;
];
