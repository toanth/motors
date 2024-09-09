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

use crate::command::UgiCommandTrait;
use crate::response::{UgiResponse, UgiResponseTrait};
use std::time::Duration;

pub struct CommandResponse<C: UgiCommandTrait, R: UgiResponseTrait> {
    command: C,
    response: Vec<R>,
    time: Duration,
}
