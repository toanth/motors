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

use rulers::engine::Engine;
use std::error::Error;
use std::io::stdin;
use std::path::Path;
use std::process::Stdio;
use std::ptr::read;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::spawn;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Enter engine path:");
    let mut engine_path = String::new();
    stdin().read_line(&mut engine_path)?;
    let mut engine = Engine::from_path(engine_path.trim())?;
    engine.send_string("ugi").await?;
    let str = engine.read_string(Duration::from_millis(3_000)).await?;
    println!("{str}");
    Ok(())
}
        
