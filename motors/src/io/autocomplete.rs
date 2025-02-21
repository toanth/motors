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
use crate::io::command::{
    coords_options, go_options, move_command, moves_options, named_entity_to_command, options_options, piece_options,
    position_options, query_options, select_command, ugi_commands, AbstractGoState, Command, CommandList, GoState,
};
use crate::io::SearchType::Normal;
use crate::io::{AbstractEngineUgi, EngineUGI, SearchType};
use crate::search::{AbstractEvalBuilder, AbstractSearcherBuilder, EvalList, SearcherList};
use edit_distance::edit_distance;
use gears::colored::Colorize;
use gears::games::OutputList;
use gears::general::board::Board;
use gears::general::common::anyhow::anyhow;
use gears::general::common::{tokens, Name, NamedEntity, Res, Tokens};
use gears::general::moves::Move;
use gears::itertools::Itertools;
use gears::output::{Message, OutputBuilder, OutputOpts};
use gears::rand::prelude::IndexedRandom;
use gears::rand::{rng, Rng};
use gears::ugi::EngineOption;
use gears::MatchStatus::Ongoing;
use gears::ProgramStatus::Run;
use gears::{ProgramStatus, Quitting};
use inquire::autocompletion::Replacement;
use inquire::{Autocomplete, CustomUserError};
use std::fmt;
use std::fmt::Debug;
use std::iter::once;
use std::rc::Rc;
use std::str::from_utf8;

fn add<T>(mut a: Vec<T>, mut b: Vec<T>) -> Vec<T> {
    a.append(&mut b);
    a
}

#[derive(Debug, Clone)]
pub struct ACState<B: Board> {
    pub go_state: GoState<B>,
    outputs: Rc<OutputList<B>>,
    searchers: Rc<SearcherList<B>>,
    evals: Rc<EvalList<B>>,
    pub(super) options: Rc<Vec<EngineOption>>,
}

impl<B: Board> ACState<B> {
    fn pos(&self) -> &B {
        &self.go_state.pos
    }
}

/// The point of this Visitor-like pattern is to minimize the amount of generic code to improve compile times:
/// It means that all commands are completely independent of the generic `Board` parameter; everything board-specific
/// is handled in this trait.
pub(super) trait AutoCompleteState: Debug {
    fn go_subcmds(&self, search_type: SearchType) -> CommandList;
    fn pos_subcmds(&self, accept_pos: bool) -> CommandList;
    fn option_subcmds(&self, only_name: bool) -> CommandList;
    fn moves_subcmds(&self, allow_moves_word: bool, recurse: bool) -> CommandList;
    fn query_subcmds(&self) -> CommandList;
    fn output_subcmds(&self) -> CommandList;
    fn print_subcmds(&self) -> CommandList;
    fn engine_subcmds(&self) -> CommandList;
    fn set_eval_subcmds(&self) -> CommandList;
    fn coords_subcmds(&self, ac_coords: bool, only_occupied: bool) -> CommandList;
    fn piece_subcmds(&self) -> CommandList;
    fn make_move(&mut self, mov: &str);
    fn options(&self) -> &[EngineOption];
}

impl<B: Board> AutoCompleteState for ACState<B> {
    fn go_subcmds(&self, search_type: SearchType) -> CommandList {
        go_options::<B>(Some(search_type))
    }
    fn pos_subcmds(&self, accept_pos: bool) -> CommandList {
        position_options(Some(self.pos()), accept_pos)
    }
    fn option_subcmds(&self, only_name: bool) -> CommandList {
        options_options(self, true, only_name)
    }
    fn moves_subcmds(&self, allow_moves_word: bool, recurse: bool) -> CommandList {
        let mut res = moves_options(self.pos(), recurse);
        if allow_moves_word {
            res.push(move_command(recurse));
        }
        res
    }
    fn query_subcmds(&self) -> CommandList {
        query_options::<B>()
    }
    fn output_subcmds(&self) -> CommandList {
        add(
            select_command::<dyn OutputBuilder<B>>(self.outputs.as_slice()),
            vec![
                named_entity_to_command(&Name {
                    short: "remove".to_string(),
                    long: "remove".to_string(),
                    description: Some("Remove the specified output, or all if not given".to_string()),
                }),
                named_entity_to_command(&Name {
                    short: "add".to_string(),
                    long: "add".to_string(),
                    description: Some("Add an output without changing existing outputs".to_string()),
                }),
            ],
        )
    }
    fn print_subcmds(&self) -> CommandList {
        add(select_command::<dyn OutputBuilder<B>>(self.outputs.as_slice()), position_options(Some(self.pos()), true))
    }
    fn engine_subcmds(&self) -> CommandList {
        select_command::<dyn AbstractSearcherBuilder<B>>(self.searchers.as_slice())
    }
    fn set_eval_subcmds(&self) -> CommandList {
        select_command::<dyn AbstractEvalBuilder<B>>(self.evals.as_slice())
    }

    fn coords_subcmds(&self, ac_coords: bool, only_occupied: bool) -> CommandList {
        coords_options(&self.go_state.pos, ac_coords, only_occupied)
    }

    fn piece_subcmds(&self) -> CommandList {
        piece_options(&self.go_state.pos)
    }

    fn make_move(&mut self, mov: &str) {
        let Ok(mov) = B::Move::from_text(mov, self.pos()) else {
            return;
        };
        if let Some(new) = self.pos().clone().make_move(mov) {
            self.go_state.pos = new;
        }
    }

    fn options(&self) -> &[EngineOption] {
        self.options.as_slice()
    }
}

impl<B: Board> AbstractEngineUgi for ACState<B> {
    fn options_text(&self, _words: &mut Tokens) -> Res<String> {
        Ok(String::new())
    }
    fn write_ugi(&mut self, _message: &fmt::Arguments) {
        /*do nothing*/
    }
    fn write_message(&mut self, _message: Message, _msg: &fmt::Arguments) {
        /*do nothing*/
    }
    fn write_response(&mut self, _msg: &str) -> Res<()> {
        Ok(())
    }
    fn status(&self) -> &ProgramStatus {
        &Run(Ongoing)
    }
    fn go_state_mut(&mut self) -> &mut dyn AbstractGoState {
        &mut self.go_state
    }

    fn load_go_state_pos(&mut self, name: &str, words: &mut Tokens) -> Res<()> {
        self.go_state.load_pos(name, words, true)
    }

    fn handle_ugi(&mut self, _proto: &str) -> Res<()> {
        Ok(())
    }
    fn handle_uginewgame(&mut self) -> Res<()> {
        Ok(())
    }
    fn handle_pos(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn handle_go(&mut self, _initial_search_type: SearchType, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn handle_stop(&mut self, _suppress_best_move: bool) -> Res<()> {
        Ok(())
    }
    fn handle_ponderhit(&mut self) -> Res<()> {
        Ok(())
    }
    fn handle_setoption(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn handle_interactive(&mut self) -> Res<()> {
        Ok(())
    }
    fn handle_debug(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn handle_log(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn handle_output(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn handle_print(&mut self, _words: &mut Tokens, _opts: OutputOpts) -> Res<()> {
        Ok(())
    }
    fn handle_engine_print(&mut self) -> Res<()> {
        Ok(())
    }
    fn handle_eval_or_tt(&mut self, _eval: bool, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn handle_engine(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn handle_set_eval(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn load_pgn(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn handle_flip(&mut self) -> Res<()> {
        self.go_state.pos = self.go_state.pos.clone().make_nullmove().ok_or(anyhow!(""))?;
        Ok(())
    }
    fn handle_query(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }
    fn handle_play(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }

    fn handle_assist(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }

    fn handle_undo(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }

    fn handle_gb(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }

    fn handle_place_piece(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }

    fn handle_remove_piece(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }

    fn handle_move_piece(&mut self, _words: &mut Tokens) -> Res<()> {
        Ok(())
    }

    fn print_help(&mut self) -> Res<()> {
        Ok(())
    }
    fn write_is_player(&mut self, _is_first: bool) -> Res<()> {
        Ok(())
    }
    fn respond_game(&mut self) -> Res<()> {
        Ok(())
    }
    fn respond_engine(&mut self) -> Res<()> {
        Ok(())
    }
    fn handle_quit(&mut self, _typ: Quitting) -> Res<()> {
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct CommandAutocomplete<B: Board> {
    // Rc because the Autocomplete trait requires DynClone and invokes `clone` on every prompt call
    pub list: Rc<CommandList>,
    pub state: ACState<B>,
}

impl<B: Board> CommandAutocomplete<B> {
    pub fn new(ugi: &EngineUGI<B>) -> Self {
        let state = ACState {
            go_state: GoState::new(ugi, Normal),
            outputs: ugi.output_factories.clone(),
            searchers: ugi.searcher_factories.clone(),
            evals: ugi.eval_factories.clone(),
            options: Rc::new(ugi.get_options()),
        };
        Self { list: Rc::new(ugi_commands()), state }
    }
}

fn distance(input: &str, name: &str) -> isize {
    if input.eq_ignore_ascii_case(name) {
        0
    } else {
        // bonus if the case matches exactly for a prefix, so `B` is more likely to be `Bb4` than `b4`.
        let bonus = if input.starts_with(name) { 1 } else { 0 };
        let lowercase_name = name.to_lowercase();
        let input = input.to_lowercase();
        let prefix = &lowercase_name.as_bytes()[..input.len().min(lowercase_name.len())];
        2 * (2 + edit_distance(&input, from_utf8(prefix).unwrap_or(name)) as isize - bonus)
    }
}

fn push(completions: &mut Vec<(isize, Completion)>, word: &str, node: &Command) {
    completions.push((
        node.autocomplete_badness(word, distance),
        Completion { name: node.short_name(), text: completion_text(node, word) },
    ));
}

/// Recursively go through all commands that have been typed so far and add completions.
/// `node` is the command we're currently looking at, `rest` are the tokens after that,
/// and `to_complete` is the last typed token or `""`, which is the one that should be completed
fn completions_for<B: Board>(
    node: &Command,
    state: &mut ACState<B>,
    rest: &mut Tokens,
    to_complete: &str,
) -> Vec<(isize, Completion)> {
    let mut res: Vec<(isize, Completion)> = vec![];
    let mut next_token = rest.peek().copied();
    // ignore all other suggestions if the last complete token requires a subcommand
    // compute this before `next_token` might be changed in the loop
    let add_subcommands = next_token.is_none_or(|n| n == to_complete) || node.autocomplete_recurse();
    loop {
        let mut found_subcommand = false;
        for child in &node.sub_commands(state) {
            // If this command is the last complete token or can recurse, add all subcommands to completions
            if add_subcommands {
                push(&mut res, to_complete, child);
            }
            // if the next token is a subcommand of this command, add suggestions for it.
            // This consumes tokens, so check all remaining subcommands again for the remaining input
            if next_token.is_some_and(|name| child.matches(name)) {
                found_subcommand = true;
                _ = rest.next(); // eat the token for the subcommand
                let mut state = state.clone();
                // possibly change the autocomplete state
                _ = child.func()(&mut state, rest, next_token.unwrap());
                let mut new_completions = completions_for(child, &mut state, rest, to_complete);
                next_token = rest.peek().copied();
                res.append(&mut new_completions);
            }
        }
        if !found_subcommand {
            break;
        }
    }
    res
}

fn underline_match(name: &str, word: &str) -> String {
    if name == word {
        format!("{}", name.underline())
    } else {
        name.to_string()
    }
}

fn completion_text(n: &Command, word: &str) -> String {
    use std::fmt::Write;
    let name = &n.primary_name;
    let mut res = format!("{}", underline_match(name, word).bold());
    for name in &n.other_names {
        write!(&mut res, " | {}", underline_match(name, word)).unwrap();
    }
    if let Some(text) = &n.help_text {
        write!(&mut res, ":  {text}").unwrap();
    }
    res
}

#[derive(Eq, PartialEq)]
struct Completion {
    name: String,
    text: String,
}

/// top-level function for completion suggestions, calls the recursive completions() function
fn suggestions<B: Board>(autocomplete: &CommandAutocomplete<B>, input: &str) -> Vec<Completion> {
    let mut words = tokens(input);
    let Some(cmd_name) = words.next() else {
        return vec![];
    };
    let to_complete =
        if input.ends_with(|s: char| s.is_whitespace()) { "" } else { input.split_whitespace().last().unwrap() };
    let complete_first_token = words.peek().is_none() && !to_complete.is_empty();

    let mut res = vec![];
    if !(complete_first_token && to_complete == "?") {
        for cmd in autocomplete.list.iter() {
            if complete_first_token {
                push(&mut res, to_complete, cmd);
            } else if cmd.matches(cmd_name) {
                let mut new = completions_for(cmd, &mut autocomplete.state.clone(), &mut words, to_complete);
                res.append(&mut new);
            }
        }
    }
    if complete_first_token {
        let moves = moves_options(autocomplete.state.pos(), false);
        for mov in &moves {
            push(&mut res, to_complete, mov);
        }
    }
    res.sort_by_key(|(val, name)| (*val, name.name.clone()));
    if let Some(min) = res.first().map(|(val, _name)| *val) {
        res.into_iter().dedup().take_while(|(val, _text)| *val <= min).map(|(_val, text)| text).collect()
    } else {
        vec![]
    }
}

impl<B: Board> Autocomplete for CommandAutocomplete<B> {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, CustomUserError> {
        Ok(suggestions(self, input).into_iter().map(|c| c.text).collect())
    }

    fn get_completion(
        &mut self,
        input: &str,
        highlighted_suggestion: Option<String>,
    ) -> Result<Replacement, CustomUserError> {
        let replacement = {
            let suggestions = suggestions(self, input);
            if let Some(suggestion) = &highlighted_suggestion {
                suggestions.into_iter().find(|s| *s.text == *suggestion).map(|s| s.name)
            } else if suggestions.len() == 1 {
                Some(suggestions[0].name.clone())
            } else {
                None
            }
        };
        if let Some(r) = replacement {
            let mut keep_words = input.split_whitespace();
            if !input.ends_with(|c: char| c.is_whitespace()) {
                keep_words = keep_words.dropping_back(1);
            }
            let res: String = keep_words.chain(once(r.as_str())).join(" ");
            Ok(Some(res))
        } else {
            Ok(None)
        }
    }
}

// Useful for generating a fuzz testing corpus
#[allow(unused)]
pub fn random_command<B: Board>(initial: String, ac: &mut CommandAutocomplete<B>, depth: usize) -> String {
    let mut res = initial;
    for i in 0..depth {
        res.push(' ');
        let s = suggestions(ac, &res);
        let s = s.choose(&mut rng());
        if rng().random_range(0..7) == 0 {
            res += &rng().random_range(-1000..10_000).to_string();
        } else if depth == 0 || s.is_none() {
            return res;
        } else {
            res += &s.unwrap().name;
        }
    }
    res
}
