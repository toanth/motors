use crate::play::ugi_client::ClientState;
use gears::games::Board;
use gears::games::Color::{Black, White};
use gears::score::Score;
use gears::{
    player_res_to_match_res, AdjudicationReason, GameOver, GameOverReason, GameResult, GameState,
    MatchResult, PlayerResult,
};

pub trait Adjudication<B: Board> {
    fn adjudicate(&mut self, state: &ClientState<B>) -> Option<MatchResult>;
}

#[derive(Debug, Default, Copy, Clone)]
pub struct ScoreAdjudication {
    pub move_number: usize,
    pub score_threshold: Score,
    pub start_after: usize,
    pub counter: usize,
}

#[derive(Debug)]
pub struct Adjudicator {
    resign: Option<ScoreAdjudication>,
    draw: Option<ScoreAdjudication>,
    max_moves_until_draw: usize,
}

impl Adjudicator {
    pub fn new(
        resign: Option<ScoreAdjudication>,
        draw: Option<ScoreAdjudication>,
        max_moves_until_draw: usize,
    ) -> Self {
        Self {
            resign,
            draw,
            max_moves_until_draw,
        }
    }

    fn adjudicate_resignation<B: Board>(&mut self, state: &ClientState<B>) -> Option<MatchResult> {
        let mut resign = self.resign?;
        if state.ply_count() < resign.start_after {
            return None;
        } else if state.ply_count() >= self.max_moves_until_draw {
            let message = format!(
                "The specified maximum of {} plies was reached",
                2 * self.max_moves_until_draw
            );
            return Some(MatchResult {
                result: GameResult::Draw,
                reason: GameOverReason::Adjudication(AdjudicationReason::Adjudicator(message)),
            });
        }
        let white_score = state
            .get_engine(White)
            .current_match
            .as_ref()
            .unwrap()
            .search_info
            .as_ref()?
            .score;
        let black_score = state
            .get_engine(Black)
            .current_match
            .as_ref()
            .unwrap()
            .search_info
            .as_ref()?
            .score;
        let mut player = None;
        let mut counter = resign.counter;
        if white_score > resign.score_threshold && black_score < -resign.score_threshold {
            counter += 1;
            player = Some(White);
        } else if white_score < -resign.score_threshold && black_score > resign.score_threshold {
            counter += 1;
            player = Some(Black);
        } else {
            counter = 0;
        }
        resign.counter = counter; // can't change resign.counter directly because that would imply two mut references
        if resign.counter >= resign.move_number {
            assert!(player.is_some());
            let message = format!(
                "Limit of {0} cp exceeded for {counter} plies in a row",
                resign.score_threshold.0
            );
            let game_over = GameOver {
                result: PlayerResult::Lose,
                reason: GameOverReason::Adjudication(AdjudicationReason::Adjudicator(message)),
            };
            let res = player_res_to_match_res(game_over, player.unwrap());
            return Some(res);
        }
        None
    }

    fn adjudicate_draw<B: Board>(&mut self, state: &ClientState<B>) -> Option<MatchResult> {
        let draw = self.draw?;
        if state.ply_count() < draw.start_after {
            // < instead of <= because start_after == 0 means to start immediately
            return None;
        }
        let mut counter = draw.counter;
        if state
            .get_engine(White)
            .current_match
            .as_ref()
            .unwrap()
            .search_info
            .as_ref()?
            .score
            .abs()
            < draw.score_threshold
            && state
                .get_engine(Black)
                .current_match
                .as_ref()
                .unwrap()
                .search_info
                .as_ref()?
                .score
                .abs()
                < draw.score_threshold
        {
            counter += 1;
        }
        if counter >= draw.move_number {
            let message = format!(
                "Both engine's score was less than {0} cp for {counter} plies in a row",
                draw.score_threshold.0
            );
            let game_over = GameOver {
                result: PlayerResult::Draw,
                reason: GameOverReason::Adjudication(AdjudicationReason::Adjudicator(message)),
            };
            return Some(player_res_to_match_res(
                game_over,
                state.the_match.board.active_player(),
            ));
        }
        None
    }
}

impl<B: Board> Adjudication<B> for Adjudicator {
    fn adjudicate(&mut self, state: &ClientState<B>) -> Option<MatchResult> {
        if state.contains_human() {
            return None;
        }
        self.adjudicate_draw(state)
            .or_else(|| self.adjudicate_resignation(state))
    }
}
