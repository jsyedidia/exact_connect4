# Alpha-Beta Search

Alpha-beta search is minimax with a score window. Instead of asking only "what
is the exact score?", a recursive call is often asked a narrower question:

```text
Can this position score at least beta?
```

The search keeps two bounds:

- `alpha` is the best score already proven for the side to move.
- `beta` is the score that would be good enough to stop searching this branch.

When a child returns `score >= beta`, the caller has found a move that is at
least good enough. It does not need to know the exact score of that child, so it
can cut off the remaining siblings.

## Exact Scores And Bounds

An exact Connect4 score says when the game is won or lost under perfect play. A
positive score is a win for the side to move; a negative score is a loss; zero
is a draw. Larger positive scores are faster wins, while more negative scores
are slower losses.

An alpha-beta call can return a bound rather than a globally exact score:

- If the result is `<= alpha`, the true score is no better than that window.
- If the result is `>= beta`, the true score is at least that good.
- If the result is strictly inside the window, it is exact for that window.

The solver's `solve` method turns bound queries into an exact score by using
repeated null-window searches. Each null-window search asks whether the score is
above one chosen threshold, then shrinks the possible score range until only one
score remains.

## Why Move Ordering Matters

Alpha-beta is correct no matter which legal move is searched first, but it is
much faster when strong moves are searched early. A good early move raises
`alpha` quickly, which makes later branches easier to cut off.

## Implemented In This Repo

The alpha-beta search lives in `src/solver.rs`, mainly in
`SolverWithTable::negamax`.

The important code-level pieces are:

- `SolverWithTable::solve` repeatedly calls `negamax` with a null window
  `(median, median + 1)` until the exact score is known.
- `SolverWithTable::negamax` receives `alpha` and `beta`, tightens them with
  mathematically reachable score bounds, and returns as soon as the window
  closes.
- A child score satisfying `score >= beta` triggers a beta cutoff. The solver
  stores that result as a lower bound in the transposition table.
- A normal exit stores the final `alpha` as an upper bound for the searched
  window.
- `MoveSorter` and `Position::move_score` support alpha-beta by making good
  cutoffs happen earlier.

The matching source notes are [notes/src/solver.rs.md](../src/solver.rs.md)
and [notes/src/move_sorter.rs.md](../src/move_sorter.rs.md).
