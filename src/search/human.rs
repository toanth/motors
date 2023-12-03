use std::fmt::{Debug, Formatter};
use std::time::{Duration, Instant};

use crate::games::{Board, ZobristHistoryBase};
use crate::search::{SearchLimit, SearchResult, Searcher, TimeControl};
use crate::ui::text_ui::TextUI;
use crate::ui::{to_ui_handle, UIHandle};

pub struct Human<B: Board> {
    ui: UIHandle<B>,
}

impl<B: Board> Human<B> {
    pub fn new(ui: UIHandle<B>) -> Self {
        Self { ui }
    }
}

impl<B: Board> Debug for Human<B> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.ui, f)
    }
}

impl<B: Board> Default for Human<B> {
    fn default() -> Self {
        Human {
            ui: to_ui_handle(TextUI::default()),
        }
    }
}

impl<B: Board> Searcher<B> for Human<B> {
    fn search(&mut self, pos: B, _: SearchLimit, _: ZobristHistoryBase) -> SearchResult<B> {
        let mut handle = self.ui.borrow_mut();
        SearchResult::move_only(handle.get_move(&pos))
    }

    fn time_up(&self, tc: TimeControl, hard_limit: Duration, start_time: Instant) -> bool {
        let elapsed = start_time.elapsed();
        elapsed > tc.remaining.min(hard_limit)
    }

    fn name(&self) -> &'static str {
        "Human"
    }
}
