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
use gears::games::{Color, PosHash, ZobristHistory};
use gears::general::board::{Board, BoardHelpers};
use gears::PlayerResult;
use itertools::Itertools;
use std::cmp::min;
use std::marker::PhantomData;
// See <https://journals.sagepub.com/doi/epdf/10.3233/ICG-2012-35302>

const INFINITY: u64 = u64::MAX / 2;

#[derive(Debug, Default, Copy, Clone)]
struct Node {
    // phi is pn (the proof number) for OR nodes and dn (disproof number) for AND nodes
    phi: u64,
    // delta is dn (the disproof number) for OR nodes and pn (proof number) for AND nodes
    delta: u64,
    hash: PosHash,
    // TODO: store ply, or subtree size, or someting along those lines for a better replacement strategy
}

struct DeltaPhi {
    phi: u64,
    delta: u64,
}

struct Pn<B: Board> {
    tt: Vec<Node>,
    root_player: B::Color,
    _phantom: PhantomData<B>,
}

impl<B: Board> Pn<B> {
    pub fn new(num_tt_entries: usize) -> Self {
        Self { tt: vec![Node::default(); num_tt_entries], root_player: B::Color::first(), _phantom: PhantomData }
    }
}

impl<B: Board> Pn<B> {
    pub fn df_pn(&mut self, pos: B) -> bool {
        self.root_player = pos.active_player();
        self.nega_dfpn(&pos, INFINITY, INFINITY);
        let dp = self.retrieve_proof_and_disproof_numbers(&pos);
        dp.delta == INFINITY
    }

    // df-pn, an improved version of pds. Eventually, this should be expanded to dfpn-pn, similar to how pds-pn works.
    fn nega_dfpn(&mut self, pos: &B, mut phi: u64, mut delta: u64) {
        // TODO: Collect into arrayvec or similar instead, or better use the smallvec crate, also for some movelists
        let children = pos.children().collect_vec();
        // TODO: Use History? Handle GHI, infinite searches, etc.
        if let Some(res) = pos.player_result(&ZobristHistory::default(), children.is_empty()) {
            let dp = self.player_res_to_deltaphi(res, pos.active_player());
            self.save_proof_and_disproof_numbers(pos, dp);
            return;
        }
        let mut phi_c = 0;
        let mut delta_2 = 0;
        loop {
            let DeltaPhi { phi: phi_sum, delta: delta_min } = self.delta_min_and_phi_sum(&children);
            if phi <= delta_min || delta <= phi_sum {
                break;
            }
            let best_child_idx = self.select_child(&children, &mut phi_c, &mut delta_2);
            let child = &children[best_child_idx];
            let child_phi = delta + phi_c - phi_sum;
            let child_delta = min(phi, delta_2 + 1);
            self.nega_dfpn(child, child_phi, child_delta);
        }
        let DeltaPhi { phi: phi_sum, delta: delta_min } = self.delta_min_and_phi_sum(&children);
        phi = delta_min;
        delta = phi_sum;
        self.save_proof_and_disproof_numbers(pos, DeltaPhi { phi, delta });
    }

    fn select_child(&mut self, children: &Vec<B>, phi_c: &mut u64, delta_2: &mut u64) -> usize {
        let mut delta_c = INFINITY;
        *phi_c = INFINITY;
        let mut best_child_idx = 0;
        for (i, c) in children.iter().enumerate() {
            let dp = self.retrieve_proof_and_disproof_numbers(c);
            if dp.delta < delta_c {
                best_child_idx = i;
                *delta_2 = delta_c;
                *phi_c = dp.phi;
                delta_c = dp.delta;
            } else if dp.delta < *delta_2 {
                *delta_2 = dp.delta;
            }
            if dp.phi == INFINITY {
                return best_child_idx;
            }
        }
        best_child_idx
    }

    fn delta_min_and_phi_sum(&self, children: &Vec<B>) -> DeltaPhi {
        let mut min = INFINITY;
        let mut sum = 0;
        for c in children {
            let dp = self.retrieve_proof_and_disproof_numbers(c);
            min = min.min(dp.delta);
            sum = (sum + dp.phi).min(INFINITY);
        }
        DeltaPhi { delta: min, phi: sum }
    }

    fn retrieve_proof_and_disproof_numbers(&self, pos: &B) -> DeltaPhi {
        if let Some(entry) = self.get(pos.hash_pos()) {
            DeltaPhi { phi: entry.phi, delta: entry.delta }
        } else {
            // TODO: This could use game-dependent knowledge, such as from an eval function
            DeltaPhi { phi: 1, delta: 1 }
        }
    }

    fn save_proof_and_disproof_numbers(&mut self, pos: &B, dp: DeltaPhi) {
        let hash = pos.hash_pos();
        let len = self.tt.len();
        let entry = &mut self.tt[hash.0 as usize % len];
        // if entry.hash != hash && entry.
        // currently, we're using always replace. TODO: Test better replacement strategy
        entry.hash = hash;
        entry.delta = dp.delta;
        entry.phi = dp.phi;
    }

    fn get(&self, hash: PosHash) -> Option<Node> {
        // TODO: Multiplication trick
        let entry = self.tt[hash.0 as usize % self.tt.len()];
        if entry.hash == hash {
            Some(entry)
        } else {
            None
        }
    }

    fn player_res_to_deltaphi(&self, res: PlayerResult, active: B::Color) -> DeltaPhi {
        match res {
            PlayerResult::Win => DeltaPhi { phi: 0, delta: INFINITY },
            // TODO: Allow proving loss, and also draw by disproving win and loss
            // TODO: For the opponent, a draw is like a win (but not for us)
            PlayerResult::Lose => DeltaPhi { phi: INFINITY, delta: 0 },
            PlayerResult::Draw => {
                let res = if self.root_player == active { PlayerResult::Lose } else { PlayerResult::Win };
                self.player_res_to_deltaphi(res, active)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gears::games::chess::Chessboard;
    use gears::general::board::Strictness::Strict;

    #[test]
    fn simple_dfpn_chess_test() {
        let pos = Chessboard::from_name("mate_in_1").unwrap();
        // TODO: Make this work with a smaller TT
        let mut searcher = Pn::new(1024 * 1024);
        let res = searcher.df_pn(pos);
        assert!(res);
        let pos = Chessboard::from_name("draw_in_1").unwrap();
        let res = searcher.df_pn(pos);
        assert!(!res);
        let pos = Chessboard::from_fen("8/8/8/1r2p3/8/1k6/8/K7 b - - 0 1", Strict).unwrap();
        let res = searcher.df_pn(pos);
        assert!(res);
        let pos = Chessboard::from_fen("8/8/8/1r2p3/8/1k6/8/K7 w - - 0 1", Strict).unwrap();
        let res = searcher.df_pn(pos);
        assert!(!res);
        let pos = Chessboard::from_fen("8/8/8/8/3p4/1k6/8/K7 b - - 0 1", Strict).unwrap();
        let res = searcher.df_pn(pos);
        assert!(res);
        let pos = Chessboard::from_fen("r2q3r/pppb3p/2n2bp1/8/3P3k/6NP/PP3PP1/R1B1R1K1 w - - 0 20", Strict).unwrap();
        let res = searcher.df_pn(pos);
        assert!(res);
        let pos = Chessboard::from_fen("rk6/p1rBK1p1/P6p/4B3/8/8/1p6/8 w - - 0 4", Strict).unwrap();
        let res = searcher.df_pn(pos);
        assert!(res);
        let pos = Chessboard::from_fen("rk6/p1rB1Kp1/P6p/4B3/8/1p6/8/8 w - - 0 3", Strict).unwrap();
        let res = searcher.df_pn(pos);
        assert!(res);
        let pos = Chessboard::from_name("puzzle").unwrap();
        let res = searcher.df_pn(pos);
        assert!(res);
    }
}
