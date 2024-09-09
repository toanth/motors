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

use crate::command::{UgiCommand, UgiCommandTrait};
use crate::response::UgiResponse;
use crate::Res;
use std::fmt::{Display, Formatter};
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use thiserror::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::time::{timeout, Instant};

pub struct Engine {
    child: Child,
    to_engine: ChildStdin,
    from_engine: Lines<BufReader<ChildStdout>>,
}

impl Engine {
    pub fn from_path(path: &str) -> Res<Self> {
        Self::new(Path::new(path), &[])
    }

    pub fn new(file: &Path, args: &[String]) -> Res<Self> {
        let mut cmd = Command::new(file);
        cmd.kill_on_drop(true)
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .args(args);
        let mut child = cmd.spawn()?;
        let stdout = child.stdout.take().unwrap();
        let to_engine = child.stdin.take().unwrap();
        let reader = BufReader::new(stdout).lines();
        Ok(Self {
            child,
            to_engine,
            from_engine: reader,
        })
    }

    pub async fn send_string(&mut self, message: &str) -> Res<()> {
        self.to_engine.write(message.as_bytes()).await?;
        self.to_engine.write(b"\n").await?;
        Ok(())
    }

    // TODO: Automatically generate to_string for ugi commands
    // pub async fn send_command<T>(&mut self, cmd: &UgiCommand<T>) -> Res<()> {
    //     self.send_string(&cmd.to_string())
    // }

    pub async fn read_string(&mut self, time_limit: Duration) -> Res<String> {
        let res = timeout(time_limit, self.from_engine.next_line())
            .await??
            .ok_or(NoNextLineError)?;
        Ok(res)
    }
}

#[derive(Debug, Error)]
struct NoNextLineError;

impl Display for NoNextLineError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Expected engine to output a line, got nothing")
    }
}
