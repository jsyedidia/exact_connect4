# Negamax

Negamax is a compact way to write minimax for two-player zero-sum games. Instead
of having separate "max player" and "min player" code paths, every position is
evaluated from the point of view of the side to move.

The key identity is:

```text
score(position) = -score(child_position)
```

After the current player makes a move, the child position belongs to the
opponent. A good score for the opponent is bad for the current player, so the
child score is negated.

In the solver, that appears as:

```rust
let score = -self.negamax(next_position, -beta, -alpha);
```

The alpha-beta window is negated and swapped for the same reason. From the
child's perspective, the opponent's lower bound becomes the current upper bound,
and the opponent's upper bound becomes the current lower bound.

## Terminal Scores

Connect4 scores encode how soon a win or loss happens. If the side to move can
win immediately, the score is:

```text
(board_size + 1 - moves_played) / 2
```

If every move allows an immediate opponent win, the score is:

```text
-(board_size - moves_played) / 2
```

Those formulas make faster wins larger and slower losses less negative.

## Bounds In The Table

The transposition table stores bounds discovered by negamax:

- A beta cutoff stores a lower bound, because the position is known to be at
  least that good.
- A normal exit stores an upper bound for the searched window.

Later searches use those cached bounds to tighten `alpha` or `beta` before
looking at child moves.

## Implemented In This Repo

Negamax is implemented in `src/solver.rs`.

The main code-level pieces are:

- `SolverWithTable::solve` handles immediate wins, chooses the initial score
  range, and asks repeated null-window questions.
- `SolverWithTable::negamax` implements the recursive search from the side to
  move's perspective.
- Child positions are created by copying `Position`, calling
  `Position::play_bitboard`, and negating the recursive result.
- The recursive call uses `-beta` and `-alpha` because the child position is
  from the opponent's perspective.
- `winning_score`, the losing-move return in `negamax`, and the draw cutoff
  near a full board implement the terminal score cases.
- `encode_upper_bound` and `encode_lower_bound` package negamax bounds for the
  transposition table.

The source note [notes/src/solver.rs.md](../src/solver.rs.md) walks through the
search in source order. [notes/concepts/alpha_beta.md](alpha_beta.md) explains
the window mechanics that sit inside the negamax recursion.
