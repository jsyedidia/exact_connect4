// SPDX-FileCopyrightText: 2017-2019 Pascal Pons <contact@gamesolver.org>
// SPDX-FileCopyrightText: 2026 Jonathan Yedidia
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

#[test]
fn help_output_mentions_supported_options() {
    let output = Command::new(exact_connect4_solver_bin())
        .arg("--help")
        .output()
        .expect("failed to run exact_connect4_solver --help");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Read Connect4 positions from standard input"));
    assert!(stdout.contains("-a"));
    assert!(stdout.contains("-w"));
    assert!(stdout.contains("-b"));
}

#[test]
fn invalid_input_is_reported_and_skipped() {
    let output = run_exact_connect4_solver(&[], "32164625\n8\n6146\n");

    assert!(output.status.success());
    assert_eq!(stdout(&output), "32164625 11\n6146 18\n");
    let stderr = stderr(&output);
    assert!(stderr.contains("Line 2:"));
    assert!(stderr.contains("invalid column digit '8'"));
}

#[test]
fn analysis_output_has_one_score_per_column() {
    let sequence = "177322644317353514472267227353611516544566";
    let output = run_exact_connect4_solver(&["-a"], &format!("{sequence}\n"));

    assert!(output.status.success());
    assert_eq!(
        stdout(&output),
        format!("{sequence} -1000 -1000 -1000 -1000 -1000 -1000 -1000\n")
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn book_override_path_is_used() {
    let book_path = std::env::current_dir()
        .unwrap()
        .join(exact_connect4::DEFAULT_OPENING_BOOK_PATH);
    let book_path = book_path.to_str().unwrap();
    let output = run_exact_connect4_solver(&["-b", book_path], "32164625\n");

    assert!(output.status.success());
    assert_eq!(stdout(&output), "32164625 11\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn weak_mode_reports_win_draw_loss_values_with_default_book() {
    let output = run_exact_connect4_solver(&["-w"], "32164625\n");

    assert!(output.status.success());
    assert_eq!(stdout(&output), "32164625 1\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn benchmark_fixture_samples_match_expected_output() {
    let fixtures = [
        include_str!("fixtures/benchmarks/Test_L1_R1"),
        include_str!("fixtures/benchmarks/Test_L1_R2"),
        include_str!("fixtures/benchmarks/Test_L1_R3"),
        include_str!("fixtures/benchmarks/Test_L2_R1"),
        include_str!("fixtures/benchmarks/Test_L2_R2"),
        include_str!("fixtures/benchmarks/Test_L3_R1"),
    ];
    let expected = fixtures
        .iter()
        .flat_map(|fixture| fixture.lines().take(16))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    let input = expected
        .lines()
        .map(|line| line.split_once(' ').unwrap().0)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";

    let output = run_exact_connect4_solver(&[], &input);

    assert!(output.status.success());
    assert_eq!(stdout(&output), expected);
    assert_eq!(stderr(&output), "");
}

fn run_exact_connect4_solver(args: &[&str], stdin: &str) -> Output {
    let mut child = Command::new(exact_connect4_solver_bin())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn exact_connect4_solver");

    child
        .stdin
        .as_mut()
        .expect("failed to open exact_connect4_solver stdin")
        .write_all(stdin.as_bytes())
        .expect("failed to write exact_connect4_solver stdin");

    child
        .wait_with_output()
        .expect("failed to wait for exact_connect4_solver")
}

fn exact_connect4_solver_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_exact_connect4_solver"))
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}
