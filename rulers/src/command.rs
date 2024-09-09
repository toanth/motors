/*
 *  Motors, a collection of games and engines.
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

use crate::Variation;
use gears::search::SearchLimit;
use gears::ugi::EngineOption;
use std::fmt::{Debug, Display, Formatter};

pub trait UgiCommandTrait: Debug + Clone + Display {
    fn display_name(&self) -> String {
        return self
            .to_string()
            .split_ascii_whitespace()
            .next()
            .unwrap()
            .to_string();
    }

    fn ugi_names(&self) -> &[String];
}

pub trait ExtraUgiCommands: Debug + Clone + Display {}

#[derive(Debug, Clone)]
pub struct InitialUgiCommand;

impl Display for InitialUgiCommand {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

#[derive(Debug, Clone)]
pub struct DebugCommand {
    value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IsReadyCommand;

#[derive(Debug, Clone)]
pub struct SetOptionCommand {
    option: EngineOption,
}

#[derive(Debug, Clone)]
pub enum PosDescription {
    Fen(String),
    Name(String),
}

#[derive(Debug, Clone)]
pub struct PositionCommand {
    pos: PosDescription,
    moves: Option<Variation>,
}

#[derive(Debug, Clone)]
pub struct GoCommand {
    limit: Option<String>,
    search_moves: Option<Vec<String>>,
    ponder: Option<()>,
}

#[derive(Debug, Clone)]
pub enum RegisterCommand {
    Later,
    Name(String),
    Code(String),
}

#[derive(Debug, Clone)]
pub struct NewGameCommand;

#[derive(Debug, Clone)]
pub struct StopCommand;

#[derive(Debug, Clone)]
pub struct PonderHitCommand;

#[derive(Debug, Clone)]
pub struct QuitCommand;

// extensions

#[derive(Debug, Clone)]
pub struct FlipCommand;

#[derive(Debug, Clone)]
pub struct QuitMatchCommand;

#[derive(Debug, Clone)]
pub struct QueryCommand {
    game_over: Option<()>,
    p1_turn: Option<()>,
    result: Option<()>,
}

#[derive(Debug, Clone)]
pub struct OptionCommand;

#[derive(Debug, Clone)]
pub struct OutputCommand {
    name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PrintCommand {
    output: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LogCommand {
    file: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EngineCommand {
    name: Option<String>,
    eval: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SetEvalCommand {
    name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PlayCommand {
    game: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PerftCommand {
    depth: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct SplitPerftCommand {
    depth: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct BenchCommand {
    limit: Option<SearchLimit>,
}

#[derive(Debug, Clone)]
pub struct EvalCommand;

#[derive(Debug, Clone)]
pub struct TTCommand;

#[derive(Debug, Clone)]
pub struct HelpCommand;

#[derive(Debug, Clone)]
pub enum UgiCommand<T: ExtraUgiCommands> {
    Empty,
    InitialUgi(InitialUgiCommand),
    Debug(DebugCommand),
    IsReady(IsReadyCommand),
    SetOption(SetOptionCommand),
    Register(RegisterCommand),
    NewGame(NewGameCommand),
    Position(PositionCommand),
    Go(GoCommand),
    Stop(StopCommand),
    PonderHit(PonderHitCommand),
    Quit(QuitCommand),
    Additional(T),
}

#[derive(Debug, Clone)]
pub enum AdditionalMotorsCommands {
    Flip(FlipCommand),
    QuitMatch(QuitMatchCommand),
    Query(QueryCommand),
    Option(OptionCommand),
    Output(OutputCommand),
    Print(PrintCommand),
    Log(LogCommand),
    Engine(EngineCommand),
    SetEval(SetEvalCommand),
    Play(PlayCommand),
    Perft(PerftCommand),
    SplitPerft(SplitPerftCommand),
    Bench(BenchCommand),
    Eval(EvalCommand),
    TT(TTCommand),
    Help(HelpCommand),
}

impl Display for AdditionalMotorsCommands {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl ExtraUgiCommands for AdditionalMotorsCommands {}
