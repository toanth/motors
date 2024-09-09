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

use crate::{Res, Variation};
use anyhow::bail;
use gears::ugi::EngineOption;
use motors::search::NodeType;
use std::fmt::{Debug, Display, Formatter};
use std::iter::Peekable;
use std::str::SplitWhitespace;

pub trait UgiResponseTrait: Debug + Clone + Display {
    fn name(&self) -> &str;
}

// trait ParseUgiCommand {
//     fn parse(words: &mut Peekable<SplitWhitespace>) -> Res<Self>;
// }

#[derive(Debug, Clone, Default)]
pub struct IdResponse {
    name: String,
    author: String,
}

// impl ParseUgiCommand for IdResponse {
//     fn parse(words: &mut Peekable<SplitWhitespace>) -> Res<Self> {
//         let mut res = IdResponse::default();
//         match words.next().ok_or_else(|| bail!("Missing token"))? {
//             "name" => {}
//             "token" => words.next(),
//             _ => bail!("unrecognized option"),
//         }
//     }
// }

#[derive(Debug, Clone)]
pub struct ProtocolResponse {
    name: String,
}

#[derive(Debug, Clone)]
pub struct UgiOkResponse;

#[derive(Debug, Clone)]
pub struct ReadyOkResponse;

#[derive(Debug, Clone)]
pub struct BestMoveResponse {
    best_move: String,
    ponder: Option<String>,
}

#[derive(Debug, Clone)]
pub enum CopyProtectionResponse {
    Ok,
    Error,
}

#[derive(Debug, Clone)]
pub struct RegistrationCheckingResponse;

#[derive(Debug, Clone)]
pub enum RegistrationStatusResponse {
    Ok,
    Later,
    Error,
}

#[derive(Debug, Clone)]
pub enum RegistrationResponse {
    Checking(RegistrationCheckingResponse),
    Status(RegistrationStatusResponse),
}

#[derive(Debug, Clone)]
pub enum Score {
    Cp(usize, NodeType),
    Mate(usize),
}

#[derive(Debug, Clone)]
pub struct InfoResponse {
    depth: Option<usize>,
    sel_depth: Option<usize>,
    time: Option<usize>,
    nodes: Option<usize>,
    pv: Option<Vec<String>>,
    multi_pv: Option<usize>,
    score: Option<Score>,
    curr_move: Option<String>,
    curr_move_number: Option<usize>,
    hash_full: Option<usize>,
    nps: Option<usize>,
    tb_hits: Option<usize>,
    sb_hits: Option<usize>,
    cpu_load: Option<usize>,
    string: Option<String>,
    refutation: Option<Variation>,
    curr_line: Option<Variation>,
}

#[derive(Debug, Clone)]
pub struct OptionResponse {
    option: EngineOption,
}

#[derive(Debug, Clone)]
pub enum UgiResponse {
    Empty,
    Id(IdResponse),
    Protocol(ProtocolResponse),
    UgiOk(UgiOkResponse),
    ReadyOk(ReadyOkResponse),
    BestMove(BestMoveResponse),
    CopyProtection(CopyProtectionResponse),
    Registration(RegistrationResponse),
    Info(InfoResponse),
    Option_(OptionResponse),
}

impl Display for UgiResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl UgiResponseTrait for UgiResponse {
    fn name(&self) -> &str {
        match self {
            UgiResponse::Empty => "",
            UgiResponse::Id(_) => "id",
            UgiResponse::Protocol(_) => "protocol",
            UgiResponse::UgiOk(_) => "ugiok",
            UgiResponse::ReadyOk(_) => "readyok",
            UgiResponse::BestMove(_) => "bestmove",
            UgiResponse::CopyProtection(_) => "copyprotection",
            UgiResponse::Registration(_) => "registration",
            UgiResponse::Info(_) => "info",
            UgiResponse::Option_(_) => "option",
        }
    }
}
