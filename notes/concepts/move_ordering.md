# Move Ordering

Move ordering is a performance technique for alpha-beta search. The solver will
eventually examine the same legal moves either way, but it often proves a bound
much earlier when promising moves are searched first.

In this project, a move is promising when it creates more possible winning
cells for the side to move. `Position::move_score` computes that local score.
`MoveSorter` stores each legal move with its score and returns the highest
score first.

The implementation is intentionally small:

- Connect4 has only seven columns, so a fixed array is enough.
- Insertion sorting is cheap because there are at most seven entries.
- No heap allocation is needed while ordering moves.
- Equal-score moves are returned newest first because equal entries are not
  shifted during insertion.

This helper does not decide which moves are legal or safe. The position code
generates legal and non-losing moves; the sorter only chooses the order in which
the search tries them.

## Implemented In This Repo

Move ordering is split between `src/position.rs`, `src/move_sorter.rs`, and
`src/solver.rs`.

The main code-level pieces are:

- `Position::possible_non_losing_moves` filters legal moves before ordering.
- `Position::move_score` scores one move by counting the winning cells it
  creates.
- `MoveSorter` stores up to `WIDTH` entries in a fixed array.
- `MoveSorter::add` insertion-sorts by score without heap allocation.
- `MoveSorter::pop` returns the highest-scoring remaining move.
- `SolverWithTable::negamax` creates a sorter for each searched position,
  inserts candidate moves, and searches them in the order returned by `pop`.
- `center_first_column_order` provides a stable center-first base order before
  move scores break ties.

The matching source notes are
[notes/src/move_sorter.rs.md](../src/move_sorter.rs.md),
[notes/src/position.rs.md](../src/position.rs.md), and
[notes/src/solver.rs.md](../src/solver.rs.md).
