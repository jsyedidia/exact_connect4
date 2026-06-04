// SPDX-FileCopyrightText: 2017-2019 Pascal Pons <contact@gamesolver.org>
// SPDX-FileCopyrightText: 2026 Jonathan Yedidia
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

const WIDTH: usize = 7;
const HEIGHT: usize = 6;
const BOARD_SIZE: usize = WIDTH * HEIGHT;
const MIN_SCORE: i32 = -(BOARD_SIZE as i32) / 2 + 3;
const MAX_SCORE: i32 = (BOARD_SIZE as i32 + 1) / 2 - 3;

const FIXTURE_PAIRS: &[(&str, &str)] = &[
    ("Test_L1_R1", "Seq_L1_R1"),
    ("Test_L1_R2", "Seq_L1_R2"),
    ("Test_L1_R3", "Seq_L1_R3"),
    ("Test_L2_R1", "Seq_L2_R1"),
    ("Test_L2_R2", "Seq_L2_R2"),
    ("Test_L3_R1", "Seq_L3_R1"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
struct BenchmarkPosition {
    sequence: String,
    score: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SequenceStatus {
    ValidNonTerminal,
    Draw,
    InvalidDigit,
    InvalidCharacter,
    FullColumn,
    WinningMove,
}

#[test]
fn benchmark_fixtures_are_well_formed() {
    for (benchmark_name, _) in FIXTURE_PAIRS {
        let positions = parse_benchmark_file(&fixture_path("benchmarks", benchmark_name));
        assert_eq!(positions.len(), 1000, "{benchmark_name}");

        for position in positions {
            assert!(
                (MIN_SCORE..=MAX_SCORE).contains(&position.score),
                "{benchmark_name}: score {} is outside [{MIN_SCORE}, {MAX_SCORE}]",
                position.score
            );
            assert_eq!(
                validate_sequence(&position.sequence),
                SequenceStatus::ValidNonTerminal,
                "{benchmark_name}: {}",
                position.sequence
            );
        }
    }
}

#[test]
fn sequence_only_fixtures_match_benchmark_first_columns() {
    for (benchmark_name, sequence_name) in FIXTURE_PAIRS {
        let benchmark_positions = parse_benchmark_file(&fixture_path("benchmarks", benchmark_name));
        let sequences = parse_sequence_file(&fixture_path("positions", sequence_name));

        assert_eq!(
            benchmark_positions.len(),
            sequences.len(),
            "{benchmark_name} and {sequence_name} should have the same length"
        );

        for (index, (benchmark, sequence)) in
            benchmark_positions.iter().zip(sequences.iter()).enumerate()
        {
            assert_eq!(
                &benchmark.sequence,
                sequence,
                "{benchmark_name} and {sequence_name} differ at line {}",
                index + 1
            );
        }
    }
}

#[test]
fn manual_sequence_fixtures_cover_expected_cases() {
    let manual_path = fixture_path("manual", "sequences.tsv");
    let cases = parse_manual_sequences(&manual_path);
    let expected_categories = BTreeSet::from([
        "valid_tiny",
        "invalid_digit",
        "invalid_character",
        "full_column",
        "early_win",
        "draw_full_board",
    ]);

    let actual_categories = cases.keys().map(String::as_str).collect::<BTreeSet<_>>();
    assert_eq!(actual_categories, expected_categories);

    assert_eq!(
        validate_sequence(cases["valid_tiny"].as_str()),
        SequenceStatus::ValidNonTerminal
    );
    assert_eq!(
        validate_sequence(cases["invalid_digit"].as_str()),
        SequenceStatus::InvalidDigit
    );
    assert_eq!(
        validate_sequence(cases["invalid_character"].as_str()),
        SequenceStatus::InvalidCharacter
    );
    assert_eq!(
        validate_sequence(cases["full_column"].as_str()),
        SequenceStatus::FullColumn
    );
    assert_eq!(
        validate_sequence(cases["early_win"].as_str()),
        SequenceStatus::WinningMove
    );
    assert_eq!(
        validate_sequence(cases["draw_full_board"].as_str()),
        SequenceStatus::Draw
    );
}

fn fixture_path(kind: &str, name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(kind)
        .join(name)
}

fn parse_benchmark_file(path: &Path) -> Vec<BenchmarkPosition> {
    let contents = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("unable to read {}: {error}", path.display());
    });

    contents
        .lines()
        .enumerate()
        .map(|(line_index, line)| parse_benchmark_line(path, line_index + 1, line))
        .collect()
}

fn parse_benchmark_line(path: &Path, line_number: usize, line: &str) -> BenchmarkPosition {
    let mut fields = line.split_whitespace();
    let sequence = fields
        .next()
        .unwrap_or_else(|| panic!("{}:{line_number}: missing sequence", path.display()));
    let score = fields
        .next()
        .unwrap_or_else(|| panic!("{}:{line_number}: missing score", path.display()));
    assert!(
        fields.next().is_none(),
        "{}:{line_number}: too many fields",
        path.display()
    );
    assert!(
        !sequence.is_empty(),
        "{}:{line_number}: empty sequence",
        path.display()
    );

    BenchmarkPosition {
        sequence: sequence.to_owned(),
        score: score.parse::<i32>().unwrap_or_else(|error| {
            panic!(
                "{}:{line_number}: invalid score {score:?}: {error}",
                path.display()
            );
        }),
    }
}

fn parse_sequence_file(path: &Path) -> Vec<String> {
    let contents = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("unable to read {}: {error}", path.display());
    });

    contents
        .lines()
        .enumerate()
        .map(|(line_index, line)| {
            assert!(
                !line.is_empty(),
                "{}:{}: empty sequence",
                path.display(),
                line_index + 1
            );
            assert_eq!(
                validate_sequence(line),
                SequenceStatus::ValidNonTerminal,
                "{}:{}: invalid sequence {line}",
                path.display(),
                line_index + 1
            );
            line.to_owned()
        })
        .collect()
}

fn parse_manual_sequences(path: &Path) -> BTreeMap<String, String> {
    let contents = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("unable to read {}: {error}", path.display());
    });

    let mut cases = BTreeMap::new();
    for (line_index, line) in contents.lines().enumerate() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let mut fields = line.splitn(3, '\t');
        let category = fields
            .next()
            .unwrap_or_else(|| panic!("{}:{}: missing category", path.display(), line_index + 1));
        let sequence = fields
            .next()
            .unwrap_or_else(|| panic!("{}:{}: missing sequence", path.display(), line_index + 1));
        assert!(
            fields.next().is_some(),
            "{}:{}: missing description",
            path.display(),
            line_index + 1
        );
        assert!(
            cases
                .insert(category.to_owned(), sequence.to_owned())
                .is_none(),
            "{}:{}: duplicate category {category}",
            path.display(),
            line_index + 1
        );
    }
    cases
}

fn validate_sequence(sequence: &str) -> SequenceStatus {
    let mut board = [[0_u8; WIDTH]; HEIGHT];
    let mut heights = [0_usize; WIDTH];
    let mut player = 1_u8;

    for byte in sequence.bytes() {
        if !byte.is_ascii_digit() {
            return SequenceStatus::InvalidCharacter;
        }
        if !(b'1'..=b'7').contains(&byte) {
            return SequenceStatus::InvalidDigit;
        }

        let column = usize::from(byte - b'1');
        let row = heights[column];
        if row >= HEIGHT {
            return SequenceStatus::FullColumn;
        }

        board[row][column] = player;
        if has_alignment(&board, column, row, player) {
            return SequenceStatus::WinningMove;
        }

        heights[column] += 1;
        player = 3 - player;
    }

    if heights.iter().sum::<usize>() == BOARD_SIZE {
        SequenceStatus::Draw
    } else {
        SequenceStatus::ValidNonTerminal
    }
}

fn has_alignment(board: &[[u8; WIDTH]; HEIGHT], column: usize, row: usize, player: u8) -> bool {
    [(1, 0), (0, 1), (1, 1), (1, -1)]
        .into_iter()
        .any(|(column_delta, row_delta)| {
            let aligned = 1
                + count_direction(board, column, row, player, column_delta, row_delta)
                + count_direction(board, column, row, player, -column_delta, -row_delta);
            aligned >= 4
        })
}

fn count_direction(
    board: &[[u8; WIDTH]; HEIGHT],
    column: usize,
    row: usize,
    player: u8,
    column_delta: isize,
    row_delta: isize,
) -> usize {
    let mut count = 0;
    let mut next_column = column.checked_add_signed(column_delta);
    let mut next_row = row.checked_add_signed(row_delta);

    while let (Some(current_column), Some(current_row)) = (next_column, next_row) {
        if current_column >= WIDTH || current_row >= HEIGHT {
            break;
        }
        if board[current_row][current_column] != player {
            break;
        }

        count += 1;
        next_column = current_column.checked_add_signed(column_delta);
        next_row = current_row.checked_add_signed(row_delta);
    }

    count
}
