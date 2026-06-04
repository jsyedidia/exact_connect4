# src/solver.rs

## Role In The System

`src/solver.rs` contains the exact game-tree search. It combines the board
operations from `Position`, the fixed-capacity `MoveSorter`, the opening book,
and the transposition table into a negamax alpha-beta solver.

The public API has two main entry points:

- `solve` returns the score of a position under perfect play.
- `analyze` returns one score per column, using `INVALID_MOVE` for columns that
  cannot be played.

The default solver loads the embedded opening book. Tests and specialized
callers can still create a no-book solver when they need direct tree search.

Related concept notes:

- [notes/concepts/solver_scores.md](../concepts/solver_scores.md)
- [notes/concepts/negamax.md](../concepts/negamax.md)
- [notes/concepts/alpha_beta.md](../concepts/alpha_beta.md)
- [notes/concepts/move_ordering.md](../concepts/move_ordering.md)
- [notes/concepts/transposition_tables.md](../concepts/transposition_tables.md)
- [notes/concepts/opening_books.md](../concepts/opening_books.md)

## Code Walkthrough

### Imports

```rust
use crate::move_sorter::MoveSorter;
use crate::opening_book::{OpeningBook, OpeningBookError};
use crate::position::{BOARD_SIZE, MAX_SCORE, MIN_SCORE, Position, WIDTH, column_mask};
use crate::transposition_table::TranspositionTable;
```

The solver needs move ordering, opening-book loading, board constants and
operations, and a transposition table for cached bounds.

### Constants And Table Type

```rust
pub const DEFAULT_TABLE_LOG_SIZE: usize = 24;

pub const INVALID_MOVE: i32 = -1000;

type SolverTable<const LOG_SIZE: usize> = TranspositionTable<u32, u8, LOG_SIZE>;

pub type Solver = SolverWithTable<DEFAULT_TABLE_LOG_SIZE>;
```

The default table has a prime capacity near `2^24`. `INVALID_MOVE` is the
analysis score for columns that are illegal. `SolverTable` fixes the partial key
and value types while keeping the log size configurable. `Solver` is the public
default-size solver type.

### Solver State

```rust
#[derive(Debug, Clone)]
pub struct SolverWithTable<const LOG_SIZE: usize> {
    transposition_table: SolverTable<LOG_SIZE>,
    opening_book: OpeningBook,
    node_count: u64,
    column_order: [usize; WIDTH],
}
```

`transposition_table` caches bounds. `opening_book` stores precomputed early
positions. `node_count` counts calls to `negamax`. `column_order` stores the
center-first column order used when equal-scored moves are searched.
`SolverWithTable` is public so tests and specialized callers can choose a
different table size.

### Constructor

```rust
impl<const LOG_SIZE: usize> SolverWithTable<LOG_SIZE> {
    #[must_use]
    pub fn new() -> Self {
        Self::with_book(
            OpeningBook::from_default_book().expect("embedded default opening book should load"),
        )
    }

    #[must_use]
    pub fn without_book() -> Self {
        Self::with_book(OpeningBook::new())
    }

    pub fn load_book_from_file(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), OpeningBookError> {
        self.opening_book = OpeningBook::new();
        self.opening_book = OpeningBook::from_file(path)?;
        Ok(())
    }

    pub fn load_book_from_bytes(&mut self, bytes: &[u8]) -> Result<(), OpeningBookError> {
        self.opening_book = OpeningBook::new();
        self.opening_book = OpeningBook::from_bytes(bytes)?;
        Ok(())
    }

    fn with_book(opening_book: OpeningBook) -> Self {
        Self {
            transposition_table: SolverTable::new(),
            opening_book,
            node_count: 0,
            column_order: center_first_column_order(),
        }
    }
```

`new` creates a solver with the embedded default opening book. `without_book`
creates a direct-search solver. The load methods replace the current book and
return explicit parse or IO errors. `with_book` is the shared constructor that
allocates the transposition table, stores the book, resets the node count, and
computes the fixed column order.

### Solving A Position

```rust
    pub fn solve(&mut self, position: &Position, weak: bool) -> i32 {
        if position.can_win_next() {
            return if weak { 1 } else { winning_score(position) };
        }

        let mut min_score = -((BOARD_SIZE - position.move_count()) as i32) / 2;
        let mut max_score = ((BOARD_SIZE + 1 - position.move_count()) as i32) / 2;
        if weak {
            min_score = -1;
            max_score = 1;
        }

        while min_score < max_score {
            let mut median = min_score + (max_score - min_score) / 2;
            if median <= 0 && min_score / 2 < median {
                median = min_score / 2;
            } else if median >= 0 && max_score / 2 > median {
                median = max_score / 2;
            }

            let mut result = self.negamax(*position, median, median + 1);
            if weak {
                result = result.signum();
            }
            if result <= median {
                max_score = result;
            } else {
                min_score = result;
            }
        }

        min_score
    }
```

`solve` handles immediate wins directly. Otherwise it searches for the exact
score by repeatedly asking `negamax` a null-window question: "is the score
greater than this median?" Weak mode narrows the initial range to win, draw, or
loss. Search results are clamped with `signum` in weak mode so exact
opening-book hits still produce `-1`, `0`, or `1`.

### Analyzing Moves

```rust
    #[must_use]
    pub fn analyze(&mut self, position: &Position, weak: bool) -> [i32; WIDTH] {
        let mut scores = [INVALID_MOVE; WIDTH];
        for (column, score) in scores.iter_mut().enumerate() {
            if !position.can_play(column) {
                continue;
            }

            if position.is_winning_move(column) {
                *score = if weak { 1 } else { winning_score(position) };
                continue;
            }

            let mut next_position = *position;
            next_position
                .play_col(column)
                .expect("column was just checked");
            *score = -self.solve(&next_position, weak);
        }
        scores
    }
```

`analyze` starts with every column marked invalid. For each legal column, it
uses the immediate-win score when possible. Otherwise it plays the move and
solves the resulting position from the opponent's point of view, then negates
that score. In weak mode, immediate winning moves are reported as `1`.

### Node Count And Reset

```rust
    #[must_use]
    pub const fn node_count(&self) -> u64 {
        self.node_count
    }

    pub fn reset(&mut self) {
        self.node_count = 0;
        self.transposition_table.reset();
    }
```

`node_count` exposes the accumulated search count. `reset` clears both the
counter and cached table entries.

### Negamax Search

```rust
    fn negamax(&mut self, position: Position, mut alpha: i32, mut beta: i32) -> i32 {
        debug_assert!(alpha < beta);
        debug_assert!(!position.can_win_next());

        self.node_count += 1;
```

`negamax` receives the position by value because `Position` is small and `Copy`.
The assertions document its preconditions. Every call increments the node
counter.

```rust
        let possible_moves = position.possible_non_losing_moves();
        if possible_moves == 0 {
            return -((BOARD_SIZE - position.move_count()) as i32) / 2;
        }

        if position.move_count() >= BOARD_SIZE - 2 {
            return 0;
        }
```

The first cutoff handles positions where every move loses immediately. The
second handles drawn endings close to a full board.

```rust
        let mut min_score = -((BOARD_SIZE - 2 - position.move_count()) as i32) / 2;
        if alpha < min_score {
            alpha = min_score;
            if alpha >= beta {
                return alpha;
            }
        }

        let mut max_score = ((BOARD_SIZE - 1 - position.move_count()) as i32) / 2;
        if beta > max_score {
            beta = max_score;
            if alpha >= beta {
                return beta;
            }
        }
```

These bounds tighten the search window to scores that are still mathematically
reachable from the current move count.

```rust
        let key = position.key();
        let cached_value = self.transposition_table.get_raw(key);
        if cached_value != 0 {
            let cached_value = i32::from(cached_value);
            let lower_bound_encoding = MAX_SCORE - MIN_SCORE + 1;
            if cached_value > lower_bound_encoding {
                min_score = cached_value + 2 * MIN_SCORE - MAX_SCORE - 2;
                if alpha < min_score {
                    alpha = min_score;
                    if alpha >= beta {
                        return alpha;
                    }
                }
            } else {
                max_score = cached_value + MIN_SCORE - 1;
                if beta > max_score {
                    beta = max_score;
                    if alpha >= beta {
                        return beta;
                    }
                }
            }
        }
```

The transposition table stores encoded upper and lower bounds. Raw table lookup
returns `0` for a miss, so a nonzero value is a cache hit that can tighten
`alpha` or `beta`; if the tightened window closes, the search can return without
visiting children.

```rust
        let book_value = self.opening_book.get(&position);
        if book_value != 0 {
            return i32::from(book_value) + MIN_SCORE - 1;
        }
```

The opening book returns a raw nonzero value when the position is stored. The
solver converts that raw value back to the normal score scale and returns it
before generating child moves.

```rust
        let mut moves = MoveSorter::new();
        for &column in self.column_order.iter().rev() {
            let move_bit = possible_moves & column_mask(column);
            if move_bit != 0 {
                moves.add(move_bit, position.move_score(move_bit));
            }
        }
```

Legal non-losing moves are inserted into the sorter. The loop walks the
center-first order backward because equal-score moves are popped newest first.

```rust
        while let Some(next_move) = moves.pop() {
            let mut next_position = position;
            next_position.play_bitboard(next_move);

            let score = -self.negamax(next_position, -beta, -alpha);
            if score >= beta {
                self.transposition_table.put(key, encode_lower_bound(score));
                return score;
            }

            if score > alpha {
                alpha = score;
            }
        }

        self.transposition_table.put(key, encode_upper_bound(alpha));
        alpha
    }
}
```

Each child score is negated because the child position is from the opponent's
perspective. A beta cutoff stores a lower bound. If all children are searched
without a cutoff, the final `alpha` is stored as an upper bound for this window.

### Default

```rust
impl<const LOG_SIZE: usize> Default for SolverWithTable<LOG_SIZE> {
    fn default() -> Self {
        Self::new()
    }
}
```

`Default` creates the same empty solver as `new`.

### Column Order

```rust
const fn center_first_column_order() -> [usize; WIDTH] {
    let mut order = [0; WIDTH];
    let mut index = 0;
    while index < WIDTH {
        let direction = 1 - 2 * (index as isize % 2);
        order[index] = (WIDTH as isize / 2 + direction * (index as isize + 1) / 2) as usize;
        index += 1;
    }
    order
}
```

This builds `[3, 2, 4, 1, 5, 0, 6]` for the standard board.

### Score Helpers

```rust
const fn winning_score(position: &Position) -> i32 {
    ((BOARD_SIZE + 1 - position.move_count()) as i32) / 2
}

fn encode_upper_bound(score: i32) -> u8 {
    (score - MIN_SCORE + 1) as u8
}

fn encode_lower_bound(score: i32) -> u8 {
    (score + MAX_SCORE - 2 * MIN_SCORE + 2) as u8
}
```

`winning_score` converts "win on this move" into the solver's exact score.
Upper bounds encode into the lower nonzero values. Lower bounds encode above
that range, so the table can distinguish the two bound kinds.

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::{
        INVALID_MOVE, Solver, SolverWithTable, center_first_column_order, encode_lower_bound,
        encode_upper_bound,
    };
    use crate::position::{MAX_SCORE, MIN_SCORE, Position, WIDTH};

    type TestSolver = SolverWithTable<12>;

    #[test]
    fn center_first_column_order_starts_in_the_middle() {
        assert_eq!(center_first_column_order(), [3, 2, 4, 1, 5, 0, 6]);
    }

    #[test]
    fn bound_encodings_are_nonzero_and_separate() {
        assert_eq!(encode_upper_bound(MIN_SCORE), 1);
        assert_eq!(encode_upper_bound(MAX_SCORE), 37);
        assert_eq!(encode_lower_bound(MIN_SCORE), 38);
        assert_eq!(encode_lower_bound(MAX_SCORE), 74);
    }

    #[test]
    fn solve_returns_immediate_winning_score() {
        let position = Position::from_sequence("121212").unwrap();
        let mut solver = TestSolver::without_book();

        assert_eq!(solver.solve(&position, false), 18);
        assert_eq!(solver.node_count(), 0);
    }

    #[test]
    fn analyze_marks_full_columns_as_invalid() {
        let position = Position::from_sequence(FULL_BOARD_DRAW).unwrap();
        let mut solver = TestSolver::without_book();

        let scores = solver.analyze(&position, true);

        assert!(scores.iter().all(|score| *score == INVALID_MOVE));
    }

    #[test]
    fn weak_solver_result_stays_in_weak_score_range() {
        let position = Position::from_sequence(FULL_BOARD_DRAW).unwrap();
        let mut solver = TestSolver::without_book();

        assert!((-1..=1).contains(&solver.solve(&position, true)));
    }

    #[test]
    fn selected_late_benchmark_positions_match_expected_scores() {
        let mut solver = TestSolver::without_book();

        for line in include_str!("../tests/fixtures/benchmarks/Test_L3_R1")
            .lines()
            .take(8)
        {
            let (sequence, expected_score) = line.split_once(' ').unwrap();
            let position = Position::from_sequence(sequence).unwrap();
            let expected_score = expected_score.parse::<i32>().unwrap();

            solver.reset();
            assert_eq!(solver.solve(&position, false), expected_score, "{sequence}");
        }
    }

    #[test]
    fn analyze_returns_one_score_per_column() {
        let position = Position::from_sequence(FULL_BOARD_DRAW).unwrap();
        let mut solver = TestSolver::without_book();

        assert_eq!(solver.analyze(&position, false).len(), WIDTH);
    }

    const FULL_BOARD_DRAW: &str = "177322644317353514472267227353611516544566";

    #[test]
    fn default_book_solves_early_fixture_positions() {
        let mut solver = Solver::new();

        for line in include_str!("../tests/fixtures/benchmarks/Test_L1_R1")
            .lines()
            .take(16)
        {
            let (sequence, expected_score) = line.split_once(' ').unwrap();
            let position = Position::from_sequence(sequence).unwrap();
            let expected_score = expected_score.parse::<i32>().unwrap();

            solver.reset();
            assert_eq!(solver.solve(&position, false), expected_score, "{sequence}");
        }
    }

    #[test]
    fn weak_default_book_result_stays_in_weak_score_range() {
        let position = Position::from_sequence("32164625").unwrap();
        let mut solver = Solver::new();

        assert_eq!(solver.solve(&position, true), 1);
    }

    #[test]
    fn weak_analyze_reports_immediate_wins_as_one() {
        let position = Position::from_sequence("121212").unwrap();
        let mut solver = TestSolver::without_book();

        let scores = solver.analyze(&position, true);

        assert_eq!(scores[0], 1);
    }
}
```

The tests use a smaller table log size to keep unit tests fast. They cover
column ordering, bound encodings, immediate wins, invalid analysis columns, weak
score range, selected no-book benchmark positions, analysis shape, default book
integration on early benchmark positions, weak book results, and weak immediate
win analysis.
