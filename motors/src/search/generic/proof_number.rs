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
use crate::eval::Eval;
use crate::eval::rand_eval::RandEval;
use crate::search::statistics::Statistics;
use crate::search::{
    AbstractSearchState, BenchResult, DEFAULT_CHECK_TIME_INTERVAL, EmptySearchStackEntry, Engine, EngineInfo,
    NoCustomInfo, PVData, SearchParams,
};
use gears::PlayerResult;
use gears::games::{BoardHistory, Color, PosHash, ZobristHistory2Fold};
use gears::general::board::{Board, BoardHelpers};
use gears::general::common::StaticallyNamedEntity;
use gears::general::move_list::MoveList;
use gears::itertools::Itertools;
use gears::score::{SCORE_LOST, SCORE_WON, Score};
use gears::search::NodeType::Exact;
use gears::search::{Depth, NodesLimit, SearchInfo, SearchResult};
use std::cmp::min;
use std::fmt::Display;
use std::time::Instant;
// See <https://journals.sagepub.com/doi/epdf/10.3233/ICG-2012-35302> and
// <https://minimax.dev/docs/ultimate/pn-search/dfpn/>

const DEFAULT_NUM_TT_ENTRIES: usize = 1024;

const INFINITY: u64 = u64::MAX / 2;

type WorkT = u64;

#[derive(Debug, Default, Copy, Clone)]
struct Node {
    bounds: DeltaPhi,
    hash: PosHash,
    work: WorkT,
    chosen_move_idx: u16,
}

#[derive(Debug, Default, Copy, Clone)]
struct DeltaPhi {
    // phi is pn (the proof number) for OR nodes and dn (disproof number) for AND nodes
    phi: u64,
    // delta is dn (the disproof number) for OR nodes and pn (proof number) for AND nodes
    delta: u64,
}

#[derive(Debug)]
pub struct ProofNumberSearcher<B: Board> {
    tt: Vec<Node>,
    root_player: B::Color,
    params: SearchParams<B>,
    start_time: Instant,
    history: ZobristHistory2Fold,
}

impl<B: Board> ProofNumberSearcher<B> {
    pub fn new(num_tt_entries: usize) -> Self {
        Self {
            tt: vec![Node::default(); num_tt_entries],
            root_player: B::Color::first(),
            params: SearchParams::default(),
            start_time: Instant::now(),
            // discard positions encountered before starting the search
            history: ZobristHistory2Fold::default(),
        }
    }
}

impl<B: Board> ProofNumberSearcher<B> {
    pub fn df_pn(&mut self, pos: B) -> Option<bool> {
        self.root_player = pos.active_player();
        self.start_time = Instant::now();
        let mut root = DeltaPhi { phi: INFINITY, delta: INFINITY };
        _ = self.multi_id(&pos, &mut root)?;
        let dp = self.load_from_tt(&pos).bounds;
        if dp.phi == 0 {
            Some(true)
        } else if dp.delta == 0 {
            Some(false)
        } else {
            None
        }
    }

    pub fn try_find_move(&mut self, pos: B) -> Option<(bool, B::Move)> {
        let win = self.df_pn(pos.clone())?;
        let node = self.load_from_tt(&pos);
        let mov = pos.legal_moves_slow().into_iter().nth(node.chosen_move_idx as usize).unwrap();
        Some((win, mov))
    }

    // df-pn, an improved version of pds. Eventually, this should be expanded to dfpn-pn, similar to how pds-pn works.
    // performs iterative deepening at each node, hence the name.
    fn multi_id(&mut self, pos: &B, node: &mut DeltaPhi) -> Option<WorkT> {
        let mut work = 1;
        let mut move_idx = usize::MAX;
        // TODO: Collect into arrayvec or similar instead, or better use the smallvec crate, also for some movelists
        let nodes = self.params.atomic.count_node();
        if nodes >= self.limit().nodes.get()
            || (nodes % DEFAULT_CHECK_TIME_INTERVAL == 0
                && (self.start_time.elapsed() >= self.params.limit.fixed_time || self.params.atomic.stop_flag()))
        {
            return None;
        }
        let mut children = pos
            .children()
            .map(|c| {
                let dp = self.load_from_tt(&c).bounds;
                (c, dp)
            })
            .collect_vec();
        // TODO: Use History? Handle GHI, infinite searches, etc.
        if let Some(res) = pos.player_result(&self.history, children.is_empty()) {
            *node = self.player_res_to_deltaphi(res, pos.active_player());
            // don't overwrite already existing entries for this position: They're either already draws or we would
            // be storing an incorrect 2fold repetition draw
            if self.get(pos.hash_pos()).is_none() {
                self.save_to_tt(pos, *node, 1, move_idx);
            }
            return Some(1);
        }
        let mut phi_c = 0;
        let mut delta_2 = 0;
        loop {
            let DeltaPhi { phi: phi_sum, delta: delta_min } = self.delta_min_and_phi_sum(&children);
            if node.phi <= delta_min || node.delta <= phi_sum {
                break;
            }
            move_idx = self.select_child(&children, &mut phi_c, &mut delta_2);
            let (child, child_node) = &mut children[move_idx];
            child_node.phi = node.delta + phi_c - phi_sum;
            child_node.delta = min(node.phi, delta_2 + 1);
            self.history.push(pos.hash_pos());
            work += self.multi_id(child, child_node)?;
            self.history.pop();
        }
        let dp = self.delta_min_and_phi_sum(&children); // TODO: Don't recompute
        *node = DeltaPhi { phi: dp.delta, delta: dp.phi };
        self.save_to_tt(pos, *node, work, move_idx);
        Some(work)
    }

    fn select_child(&mut self, children: &[(B, DeltaPhi)], phi_c: &mut u64, delta_2: &mut u64) -> usize {
        let mut delta_c = INFINITY;
        *phi_c = INFINITY;
        let mut best_child_idx = 0;
        for (i, (_, dp)) in children.iter().enumerate() {
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

    fn delta_min_and_phi_sum(&self, children: &[(B, DeltaPhi)]) -> DeltaPhi {
        let mut min = INFINITY;
        let mut sum = 0;
        for (_, dp) in children {
            min = min.min(dp.delta);
            sum = (sum + dp.phi).min(INFINITY);
        }
        DeltaPhi { delta: min, phi: sum }
    }

    fn load_from_tt(&self, pos: &B) -> Node {
        if let Some(entry) = self.get(pos.hash_pos()) {
            entry
        } else {
            // TODO: This could use game-dependent knowledge, such as from an eval function
            Node { bounds: DeltaPhi { phi: 1, delta: 1 }, hash: pos.hash_pos(), work: 0, chosen_move_idx: u16::MAX }
        }
    }

    fn tt_idx(&self, hash: PosHash) -> usize {
        // Uses lemire's multiplication trick, like the TT implementation
        ((hash.0 as u128 * self.tt.len() as u128) >> 64) as usize
    }

    fn save_to_tt(&mut self, pos: &B, dp: DeltaPhi, new_work: WorkT, move_idx: usize) {
        let hash = pos.hash_pos();
        let idx = self.tt_idx(pos.hash_pos());
        let entry = &mut self.tt[idx];
        // currently, we're using always replace. TODO: Test better replacement strategy
        if !(entry.hash == hash && move_idx == usize::MAX) {
            entry.chosen_move_idx = move_idx as u16;
        }
        if entry.hash != hash {
            entry.work = 0;
        }
        entry.work += new_work;
        entry.hash = hash;
        entry.bounds.delta = dp.delta;
        entry.bounds.phi = dp.phi;
    }

    fn get(&self, hash: PosHash) -> Option<Node> {
        let entry = self.tt[self.tt_idx(hash)];
        if entry.hash == hash { Some(entry) } else { None }
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

    fn reconstruct_pv(&self, root: &B) -> Vec<B::Move> {
        let mut pos = root.clone();
        let mut res = vec![];
        let mut seen = vec![];
        loop {
            let node = self.load_from_tt(&pos);
            let moves = pos.legal_moves_slow();
            if node.hash != pos.hash_pos()
                || node.chosen_move_idx as usize >= moves.num_moves()
                || seen.contains(&pos.hash_pos())
            {
                break;
            }
            seen.push(pos.hash_pos());
            let mov = moves.into_iter().nth(node.chosen_move_idx as usize).unwrap();
            res.push(mov);
            pos = pos.make_move(mov).unwrap();
        }
        res
    }
}

impl<B: Board> StaticallyNamedEntity for ProofNumberSearcher<B> {
    fn static_short_name() -> impl Display {
        "proof"
    }

    fn static_long_name() -> String {
        "Proof Number Searcher".to_string()
    }

    fn static_description() -> String {
        "Tries to find a forced checkmate".to_string()
    }
}

impl<B: Board> AbstractSearchState<B> for ProofNumberSearcher<B> {
    fn forget(&mut self, hard: bool) {
        if hard {
            for entry in &mut self.tt {
                *entry = Node::default();
            }
        }
    }

    fn new_search(&mut self, params: SearchParams<B>) {
        self.params = params;
    }

    fn end_search(&mut self, res: &SearchResult<B>) {
        // normal searchers spin until they receive an explicit `stop` when asked to do an infinite search,
        // but this isn't useful for a proof number search.
        self.params.atomic.set_stop(true);
        self.params.end_and_send(res);
    }

    fn search_params(&self) -> &SearchParams<B> {
        &self.params
    }

    fn pv_data(&self) -> &[PVData<B>] {
        &[]
    }

    fn to_bench_res(&self) -> BenchResult {
        BenchResult::default()
    }

    fn to_search_info(&self) -> SearchInfo<B> {
        SearchInfo::default()
    }

    fn aggregated_statistics(&self) -> Statistics {
        Statistics::default()
    }

    fn send_search_info(&self) {
        // do nothing
    }

    fn write_internal_info(&self, _pos: &B) -> Option<String> {
        None
    }
}

impl<B: Board> Engine<B> for ProofNumberSearcher<B> {
    type SearchStackEntry = EmptySearchStackEntry;
    type CustomInfo = NoCustomInfo;

    fn with_eval(_eval: Box<dyn Eval<B>>) -> Self
    where
        Self: Sized,
    {
        // TODO: Use eval for leaf proof numbers
        Self::new(DEFAULT_NUM_TT_ENTRIES)
    }

    fn static_eval(&mut self, _pos: &B, _ply: usize) -> Score {
        // TODO: Use eval
        Score(0)
    }

    fn max_bench_depth(&self) -> Depth {
        Depth::new(1)
    }

    fn search_state_dyn(&self) -> &dyn AbstractSearchState<B> {
        self
    }

    fn search_state_mut_dyn(&mut self) -> &mut dyn AbstractSearchState<B> {
        self
    }

    fn engine_info(&self) -> EngineInfo {
        EngineInfo::new(self, &RandEval::default(), "0.1", Depth::new(1), NodesLimit::new(1).unwrap(), None, vec![])
    }

    fn set_eval(&mut self, _eval: Box<dyn Eval<B>>) {
        // TODO: Don't ignore the eval
    }

    fn do_search(&mut self) -> SearchResult<B> {
        let root = self.params.pos.clone();
        // TODO: Replace by having the TT type depend on the engine
        self.tt.resize(self.params.tt.size_in_bytes() / size_of::<Node>(), Node::default());
        let res = self.df_pn(root.clone());
        if let Some(res) = res {
            let score = if res { SCORE_WON } else { SCORE_LOST };

            let pv = self.reconstruct_pv(&root);
            let mov = pv[0];
            if let Some(mut o) = self.params.thread_type.output() {
                let info = SearchInfo {
                    best_move_of_all_pvs: mov,
                    iterations: 1,
                    depth: Depth::new(1),
                    seldepth: Depth::new(1),
                    time: self.start_time.elapsed(),
                    nodes: NodesLimit::new(self.params.atomic.nodes()).unwrap(),
                    pv_num: 0,
                    max_num_pvs: 1,
                    pv: &pv,
                    score: Default::default(),
                    hashfull: 0,
                    pos: root,
                    bound: Some(Exact),
                    num_threads: 1,
                    additional: None,
                };
                o.write_search_info(info);
            }
            self.send_ugi(&format_args!("Position is {}won!", if res { "" } else { "NOT " }));
            SearchResult::new(mov, score, None, self.params.pos.clone())
        } else {
            self.send_ugi(&format_args!("Failed to prove or disprove win"));
            SearchResult::new(B::Move::default(), Score(0), None, self.params.pos.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gears::games::chess::Chessboard;
    use gears::games::chess::moves::ChessMove;
    use gears::general::board::Strictness::Strict;
    use gears::general::moves::Move;

    #[test]
    fn simple_dfpn_chess_test() {
        let pos = Chessboard::from_name("mate_in_1").unwrap();
        let mut searcher = ProofNumberSearcher::new(1024 * 1024);
        let res = searcher.try_find_move(pos);
        // Rh6 leads to a variation where every move of the opponent is forced, so it's considered equally expensive as a mate in 1
        // (Rh4 would too, but that would result in a repeated position)
        let acceptable = [ChessMove::from_text("Ra7#", &pos).unwrap(), ChessMove::from_text("Rh6", &pos).unwrap()];
        assert!(matches!(res, Some((true, _))), "{res:?}");
        assert!(acceptable.contains(&res.unwrap().1), "{}", res.unwrap().1.compact_formatter(&pos));
        let pos = pos.make_nullmove().unwrap();
        let res = searcher.try_find_move(pos);
        assert!(matches!(res, Some((false, _))));
        let pos = Chessboard::from_name("draw_in_1").unwrap();
        let res = searcher.df_pn(pos);
        assert_eq!(res, Some(false));
        let pos = Chessboard::from_fen("8/8/8/1r2p3/8/1k6/8/K7 b - - 0 1", Strict).unwrap();
        let res = searcher.try_find_move(pos);
        assert!(matches!(res, Some((true, _))));
        let pos = Chessboard::from_fen("8/8/8/1r2p3/8/1k6/8/K7 w - - 0 1", Strict).unwrap();
        let res = searcher.try_find_move(pos);
        assert_eq!(res, Some((false, ChessMove::from_text("Kb1", &pos).unwrap())));
        let pos = Chessboard::from_fen("8/8/8/8/3p4/1k6/8/K7 b - - 0 1", Strict).unwrap();
        let res = searcher.try_find_move(pos);
        assert_eq!(res, Some((true, ChessMove::from_text("d3", &pos).unwrap())));
        let pos = Chessboard::from_fen("r2q3r/pppb3p/2n2bp1/8/3P3k/6NP/PP3PP1/R1B1R1K1 w - - 0 20", Strict).unwrap();
        let res = searcher.try_find_move(pos);
        assert_eq!(res, Some((true, ChessMove::from_text("Re4+", &pos).unwrap())));
        let pos = Chessboard::from_fen("rk6/p1rBK1p1/P6p/4B3/8/8/1p6/8 w - - 0 4", Strict).unwrap();
        let res = searcher.try_find_move(pos);
        assert_eq!(res, Some((true, ChessMove::from_text("Kd8", &pos).unwrap())));
        let pos = Chessboard::from_fen("rk6/p1rB1Kp1/P6p/4B3/8/1p6/8/8 w - - 0 3", Strict).unwrap();
        let res = searcher.df_pn(pos);
        assert_eq!(res, Some(true));
        let pos = Chessboard::from_name("puzzle").unwrap();
        let res = searcher.try_find_move(pos);
        assert_eq!(res, Some((true, ChessMove::from_text("Bd7", &pos).unwrap())));
    }

    #[test]
    fn tt_size_1_test() {
        let pos = Chessboard::from_name("mate_in_1").unwrap();
        let mut searcher = ProofNumberSearcher::new(1);
        let res = searcher.try_find_move(pos);
        let acceptable = [ChessMove::from_text("Ra7#", &pos).unwrap(), ChessMove::from_text("Rh6", &pos).unwrap()];
        assert!(matches!(res, Some((true, _))));
        assert!(acceptable.contains(&res.unwrap().1), "{}", res.unwrap().1.compact_formatter(&pos));
        let pos = pos.make_nullmove().unwrap();
        let res = searcher.try_find_move(pos);
        assert!(matches!(res, Some((false, _))));
    }
}
