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

use itertools::Itertools;
use std::default::Default;

const HEIGHT: usize = 6;

// TODO: Not pub
struct Letter {
    text: [String; HEIGHT],
}

impl Letter {
    pub fn new(text: &str) -> Self {
        let lines = text.lines();
        assert_eq!(lines.clone().count(), HEIGHT);
        assert!(
            lines.clone().map(|l| l.chars().count()).all_equal(),
            "{:?}",
            lines.map(|l| l.chars().count()).collect_vec()
        );
        let mut text: [String; HEIGHT] = Default::default();
        for (i, line) in lines.enumerate() {
            text[i] = line.to_string();
        }
        Self { text }
    }
}

fn letters() -> [Letter; 29] {
    [
        Letter::new(" █████╗ \n██╔══██╗\n███████║\n██╔══██║\n██║  ██║\n╚═╝  ╚═╝\n"), // A
        Letter::new("██████╗ \n██╔══██╗\n██████╔╝\n██╔══██╗\n██████╔╝\n╚═════╝ \n"), // B
        Letter::new(" ██████╗\n██╔════╝\n██║     \n██║     \n╚██████╗\n ╚═════╝\n"), // C
        Letter::new("██████╗ \n██╔══██╗\n██║  ██║\n██║  ██║\n██████╔╝\n╚═════╝ \n"), // D
        Letter::new("███████╗\n██╔════╝\n█████╗  \n██╔══╝  \n███████╗\n╚══════╝\n"), // E
        Letter::new("███████╗\n██╔════╝\n█████╗  \n██╔══╝  \n██║     \n╚═╝     \n"), // F
        Letter::new(" ██████╗ \n██╔════╝ \n██║  ███╗\n██║   ██║\n╚██████╔╝\n ╚═════╝ \n"), // G
        Letter::new("██╗  ██╗\n██║  ██║\n███████║\n██╔══██║\n██║  ██║\n╚═╝  ╚═╝\n"), // H
        Letter::new("██╗\n██║\n██║\n██║\n██║\n╚═╝\n"),                               // I
        Letter::new("     ██╗\n     ██║\n     ██║\n██   ██║\n╚█████╔╝\n ╚════╝ \n"), // J
        Letter::new("██╗  ██╗\n██║ ██╔╝\n█████╔╝ \n██╔═██╗ \n██║  ██╗\n╚═╝  ╚═╝\n"), // K
        Letter::new("██╗     \n██║     \n██║     \n██║     \n███████╗\n╚══════╝\n"), // L
        Letter::new(
            "███╗   ███╗\n████╗ ████║\n██╔████╔██║\n██║╚██╔╝██║\n██║ ╚═╝ ██║\n╚═╝     ╚═╝\n",
        ), // M
        Letter::new("███╗   ██╗\n████╗  ██║\n██╔██╗ ██║\n██║╚██╗██║\n██║ ╚████║\n╚═╝  ╚═══╝\n"), // N
        Letter::new(" ██████╗ \n██╔═══██╗\n██║   ██║\n██║   ██║\n╚██████╔╝\n ╚═════╝ \n"), // O
        Letter::new("██████╗ \n██╔══██╗\n██████╔╝\n██╔═══╝ \n██║     \n╚═╝     \n"),       // P
        Letter::new(" ██████╗ \n██╔═══██╗\n██║   ██║\n██║▄▄ ██║\n╚██████╔╝\n ╚══▀▀═╝ \n"), // Q
        Letter::new("██████╗ \n██╔══██╗\n██████╔╝\n██╔══██╗\n██║  ██║\n╚═╝  ╚═╝\n"),       // R
        Letter::new("███████╗\n██╔════╝\n███████╗\n╚════██║\n███████║\n╚══════╝\n"),       // S
        Letter::new("████████╗\n╚══██╔══╝\n   ██║   \n   ██║   \n   ██║   \n   ╚═╝   \n"), // T
        Letter::new("██╗   ██╗\n██║   ██║\n██║   ██║\n██║   ██║\n╚██████╔╝\n ╚═════╝ \n"), // U
        Letter::new("██╗   ██╗\n██║   ██║\n██║   ██║\n╚██╗ ██╔╝\n ╚████╔╝ \n  ╚═══╝  \n"), // V
        Letter::new("██╗    ██╗\n██║    ██║\n██║ █╗ ██║\n██║███╗██║\n╚███╔███╔╝\n ╚══╝╚══╝ \n"), // W
        Letter::new("██╗  ██╗\n╚██╗██╔╝\n ╚███╔╝ \n ██╔██╗ \n██╔╝ ██╗\n╚═╝  ╚═╝\n"), // X
        Letter::new("██╗   ██╗\n╚██╗ ██╔╝\n ╚████╔╝ \n  ╚██╔╝  \n   ██║   \n   ╚═╝   \n"), // Y
        Letter::new("███████╗\n╚══███╔╝\n  ███╔╝ \n ███╔╝  \n███████╗\n╚══════╝\n"), // Z
        Letter::new("      \n      \n█████╗\n╚════╝\n      \n      \n"),             // -
        Letter::new("        \n        \n        \n        \n        \n        \n"), // space
        Letter::new("       \n    ██╗\n    ╚═╝\n    ██╗\n    ╚═╝\n       \n"),       // :
    ]
}

pub fn try_print_as_ascii_art(text: &str, indent: usize) -> Option<String> {
    let idx = |c: char| {
        if c.is_ascii_alphabetic() {
            Some(c.to_ascii_lowercase().to_ascii_lowercase() as usize - b'a' as usize)
        } else if c == '-' {
            Some(26)
        } else if c == ' ' {
            Some(27)
        } else if c == ':' {
            Some(28)
        } else {
            None
        }
    };
    if text.chars().any(|c| idx(c).is_none()) {
        return None;
    }
    let letters = letters();
    let letters = text.chars().map(|c| &letters[idx(c).unwrap()]);
    let mut lines = [(); HEIGHT].map(|_| " ".repeat(indent));
    for letter in letters {
        for (line, letter_line) in lines.iter_mut().zip(letter.text.iter()) {
            *line += letter_line.as_str();
        }
    }
    Some(format!("\n{}\n", lines.join("\n")))
}

pub fn print_as_ascii_art(text: &str, indent: usize) -> String {
    try_print_as_ascii_art(text, indent)
        .unwrap_or(text.chars().map(|c| c.to_ascii_uppercase()).join(" "))
}
