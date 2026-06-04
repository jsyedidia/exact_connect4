# Connect4 Rules

This note explains the game rules that matter for the solver and its test
fixtures.

Connect4 is played on a board with 7 columns and 6 rows. Players alternate
dropping discs into columns. A disc falls to the lowest empty cell in the
chosen column.

The first player to make four connected discs wins. Connections can be
vertical, horizontal, or diagonal. If all 42 cells are filled and neither
player has made four in a row, the game is a draw.

## Move Sequences

This repo uses Pascal Pons' move-sequence notation. A position is written as a
string of 1-based column digits:

```text
4453
```

That sequence means:

1. player 1 plays column 4,
2. player 2 plays column 4,
3. player 1 plays column 5,
4. player 2 plays column 3.

Only digits `1` through `7` are legal. A sequence is invalid if it contains
another character, names a full column, or continues through a move that would
already win the game.

## Positions vs. Finished Games

Pascal Pons' `Position` type represents positions for search. It assumes that
no player has already connected four. This is why the reference parser stops
before accepting a winning move.

For example, the sequence below would give player 1 four discs in column 1 on
the last move:

```text
1212121
```

That is a valid game prefix up to the win, but it is not a valid solver
position after the final digit.

## Fixture Sets

The tracked `tests/fixtures/benchmarks/` files contain move sequences paired
with exact solver scores. The matching `tests/fixtures/positions/` files
contain only the move-sequence column. The sequence-only files are useful for
commands that ask a solver to analyze every legal move, such as:

```sh
cargo run --release --bin exact_connect4_solver -- -a < tests/fixtures/positions/Seq_L1_R2
```

That command asks the command-line solver to print one score per column for
each sequence in the file.

## Implemented In This Repo

The game rules are enforced mostly by `src/position.rs`.

The main code-level pieces are:

- `WIDTH`, `HEIGHT`, and `BOARD_SIZE` define the standard 7x6 board.
- `Position::from_sequence` and `Position::play_sequence` parse 1-based move
  strings.
- `parse_column_character` rejects non-digits and digits outside `1..=7`.
- `Position::can_play` rejects moves into full columns.
- `Position::is_winning_move` detects a move that would finish the game.
- `PlaySequenceErrorKind` distinguishes invalid characters, invalid columns,
  full columns, and winning moves.
- `tests/fixtures/benchmarks/` stores scored positions, while
  `tests/fixtures/positions/` stores sequence-only inputs.
- `src/bin/exact_connect4_solver.rs` reads one sequence per input line and reports invalid
  lines without stopping the whole process.

The related source notes are [notes/src/position.rs.md](../src/position.rs.md)
and [notes/src/bin/exact_connect4_solver.rs.md](../src/bin/exact_connect4_solver.rs.md).
