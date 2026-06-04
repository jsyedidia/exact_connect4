# Solver Scores

Pascal Pons' solver does more than classify positions as win, loss, or draw.
It computes an exact score that says how quickly the game ends under perfect
play.

Scores are always from the point of view of the player to move:

- a positive score means the player to move can force a win,
- zero means the position is a draw with perfect play,
- a negative score means the player to move loses with perfect play.

The absolute value describes the distance to the end of the game in moves.
Faster wins get larger positive scores, and slower losses get less negative
scores because the losing player delays defeat as long as possible.

To understand the scores, notice that each player has 21 stones
at the beginning of the game. If you win with your very last stone, you get a score of +1.
Otherwise, you get a score of 1 plus the number of stones remaining in your hand when you win.
The loser gets a negative score that is the negative of the winner's score. The fastest
you can win is with your fourth move, which would leave you with 17 stones in hand and a score of +18.

## Score Bounds

For the standard 7x6 board, Pascal Pons' final solver uses these bounds:

```text
MIN_SCORE = -18
MAX_SCORE = 18
```

The tracked benchmark fixtures should only contain scores in that range.

## Benchmark Format

Each scored benchmark line has this shape:

```text
<move-sequence> <score>
```

For example:

```text
32164625 11
```

The first field is the position in 1-based column notation. The second field is
the exact score for the player to move in that position.

The matching sequence-only fixture contains just the first field:

```text
32164625
```

These two fixture forms support different workflows. Scored benchmark files are
for correctness checks. Sequence-only files are useful when asking a solver to
print one score per legal move for many positions.

## Invalid Move Marker

The command-line analysis mode prints one score per column. Illegal moves use
the same marker as the C++ reference:

```text
-1000
```

That value is not an exact game score. It is a sentinel for an unplayable
column.

## Implemented In This Repo

Score handling is shared by `src/position.rs`, `src/solver.rs`, the CLI, and
the generator.

The main code-level pieces are:

- `MIN_SCORE` and `MAX_SCORE` in `src/position.rs` define the legal exact-score
  range for a 7x6 board.
- `SolverWithTable::solve` returns the exact score for the side to move.
- `winning_score` computes the immediate-win score from `BOARD_SIZE` and the
  current move count.
- `SolverWithTable::negamax` returns negative terminal scores when every legal
  move loses immediately and returns zero near drawn endings.
- `encode_upper_bound` and `encode_lower_bound` convert score bounds into
  nonzero transposition-table values.
- `OpeningBook` values use the same offset convention as upper-bound table
  values: `score = raw_value + MIN_SCORE - 1`.
- `INVALID_MOVE` in `src/solver.rs` is the `-1000` marker used by analysis
  output for unplayable columns.
- `src/bin/exact_connect4_solver.rs` prints either one exact score or seven per-column
  scores.
- `src/bin/generator.rs` encodes scored input lines into opening-book values.

The related source notes are [notes/src/solver.rs.md](../src/solver.rs.md),
[notes/src/opening_book.rs.md](../src/opening_book.rs.md),
[notes/src/bin/exact_connect4_solver.rs.md](../src/bin/exact_connect4_solver.rs.md), and
[notes/src/bin/generator.rs.md](../src/bin/generator.rs.md).
