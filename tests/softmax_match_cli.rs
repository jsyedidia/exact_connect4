// SPDX-FileCopyrightText: 2026 Jonathan Yedidia
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::PathBuf;
use std::process::{Command, Output};

#[test]
fn invalid_start_position_exits_with_error() {
    let output = Command::new(softmax_match_bin())
        .args(["--start", "1212121"])
        .output()
        .expect("failed to run softmax_match");

    assert!(!output.status.success());
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains("Invalid starting position"));
}

#[test]
fn deterministic_short_match_smoke_test() {
    let output = Command::new(softmax_match_bin())
        .args([
            "--games",
            "1",
            "--seed",
            "1",
            "--weak",
            "--start",
            "76642741435573447574271232313253652656111",
            "--verbose",
        ])
        .output()
        .expect("failed to run softmax_match");

    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        "Game 1: first player = Bot 1, winner = draw, score for Bot 1 = 0, moves = 766427414355734475742712323132536526561116\n\
Bot 1 temperature: 0\n\
Bot 2 temperature: 0\n\
Games played: 1\n\
Bot 1 wins: 0\n\
Bot 2 wins: 0\n\
Draws: 1\n\
Average score for Bot 1: 0.000\n"
    );
    assert_eq!(stderr(&output), "");
}

fn softmax_match_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_softmax_match"))
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}
