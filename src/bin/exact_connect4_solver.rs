// SPDX-FileCopyrightText: 2017-2019 Pascal Pons <contact@gamesolver.org>
// SPDX-FileCopyrightText: 2026 Jonathan Yedidia
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{io, io::BufRead};

use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = "exact_connect4_solver",
    about = "Exact Connect4 solver",
    long_about = "Read Connect4 positions from standard input, one per line.\n\n\
The embedded default opening book is loaded automatically. Use -b FILE to \
load a different compatible book file."
)]
struct Options {
    /// Print the score of every column instead of the single best-play score.
    #[arg(short = 'a')]
    analyze: bool,

    /// Use the weak solver.
    #[arg(short = 'w')]
    weak: bool,

    /// Opening book file.
    #[arg(short = 'b', value_name = "FILE", default_value = exact_connect4::DEFAULT_OPENING_BOOK_PATH)]
    opening_book: PathBuf,
}

fn main() -> ExitCode {
    let options = Options::parse();
    let mut solver =
        if options.opening_book.as_path() == Path::new(exact_connect4::DEFAULT_OPENING_BOOK_PATH) {
            exact_connect4::solver::Solver::new()
        } else {
            let mut solver = exact_connect4::solver::Solver::without_book();
            if let Err(error) = solver.load_book_from_file(&options.opening_book) {
                eprintln!(
                    "Unable to load opening book {}: {error}",
                    options.opening_book.display()
                );
            }
            solver
        };
    let stdin = io::stdin();

    for (line_number, line) in stdin.lock().lines().enumerate() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                eprintln!("Line {}: {error}", line_number + 1);
                return ExitCode::from(1);
            }
        };
        let sequence = line.trim_end_matches('\r');

        let position = match exact_connect4::position::Position::from_sequence(sequence) {
            Ok(position) => position,
            Err(error) => {
                eprintln!("Line {}: {error} in {sequence:?}", line_number + 1);
                continue;
            }
        };

        print!("{sequence}");
        if options.analyze {
            for score in solver.analyze(&position, options.weak) {
                print!(" {score}");
            }
        } else {
            print!(" {}", solver.solve(&position, options.weak));
        }
        println!();
    }

    ExitCode::SUCCESS
}
