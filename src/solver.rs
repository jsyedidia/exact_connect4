// SPDX-FileCopyrightText: 2017-2019 Pascal Pons <contact@gamesolver.org>
// SPDX-FileCopyrightText: 2026 Jonathan Yedidia
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::move_sorter::MoveSorter;
use crate::opening_book::{OpeningBook, OpeningBookError};
use crate::position::{BOARD_SIZE, MAX_SCORE, MIN_SCORE, Position, WIDTH, column_mask};
use crate::transposition_table::TranspositionTable;

/// Default transposition-table log size used by the solver.
pub const DEFAULT_TABLE_LOG_SIZE: usize = 24;

/// Score used by `analyze` for columns that cannot be played.
pub const INVALID_MOVE: i32 = -1000;

type SolverTable<const LOG_SIZE: usize> = TranspositionTable<u32, u8, LOG_SIZE>;

/// Solver using the default transposition-table size.
pub type Solver = SolverWithTable<DEFAULT_TABLE_LOG_SIZE>;

/// Exact Connect4 solver using negamax alpha-beta search.
#[derive(Debug, Clone)]
pub struct SolverWithTable<const LOG_SIZE: usize> {
    transposition_table: SolverTable<LOG_SIZE>,
    opening_book: OpeningBook,
    node_count: u64,
    column_order: [usize; WIDTH],
}

impl<const LOG_SIZE: usize> SolverWithTable<LOG_SIZE> {
    /// Creates a solver with an empty transposition table and the default book.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut solver = exact_connect4::Solver::new();
    /// let position = exact_connect4::Position::from_sequence("32164625")?;
    ///
    /// assert_eq!(solver.solve(&position, false), 11);
    /// # Ok::<(), exact_connect4::PlaySequenceError>(())
    /// ```
    #[must_use]
    pub fn new() -> Self {
        Self::with_book(
            OpeningBook::from_default_book().expect("embedded default opening book should load"),
        )
    }

    /// Creates a solver without an opening book.
    ///
    /// This is mainly useful for tests or for measuring raw tree search.
    #[must_use]
    pub fn without_book() -> Self {
        Self::with_book(OpeningBook::new())
    }

    /// Loads an opening book from a file, replacing any currently loaded book.
    pub fn load_book_from_file(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<(), OpeningBookError> {
        self.opening_book = OpeningBook::new();
        self.opening_book = OpeningBook::from_file(path)?;
        Ok(())
    }

    /// Loads an opening book from binary bytes, replacing any currently loaded book.
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

    /// Solves a position under perfect play.
    ///
    /// When `weak` is `true`, the result is only win, draw, or loss, in the
    /// range `-1..=1`.
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

    /// Scores every column from the current position.
    ///
    /// Unplayable columns are reported as `INVALID_MOVE`.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut solver = exact_connect4::Solver::new();
    /// let position = exact_connect4::Position::from_sequence(
    ///     "177322644317353514472267227353611516544566",
    /// )?;
    ///
    /// assert_eq!(solver.analyze(&position, true), [exact_connect4::INVALID_MOVE; exact_connect4::position::WIDTH]);
    /// # Ok::<(), exact_connect4::PlaySequenceError>(())
    /// ```
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

    /// Returns the number of negamax nodes searched since the last reset.
    #[must_use]
    pub const fn node_count(&self) -> u64 {
        self.node_count
    }

    /// Clears the node counter and transposition table.
    pub fn reset(&mut self) {
        self.node_count = 0;
        self.transposition_table.reset();
    }

    fn negamax(&mut self, position: Position, mut alpha: i32, mut beta: i32) -> i32 {
        debug_assert!(alpha < beta);
        debug_assert!(!position.can_win_next());

        self.node_count += 1;

        let possible_moves = position.possible_non_losing_moves();
        if possible_moves == 0 {
            return -((BOARD_SIZE - position.move_count()) as i32) / 2;
        }

        if position.move_count() >= BOARD_SIZE - 2 {
            return 0;
        }

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

        let book_value = self.opening_book.get(&position);
        if book_value != 0 {
            return i32::from(book_value) + MIN_SCORE - 1;
        }

        let mut moves = MoveSorter::new();
        for &column in self.column_order.iter().rev() {
            let move_bit = possible_moves & column_mask(column);
            if move_bit != 0 {
                moves.add(move_bit, position.move_score(move_bit));
            }
        }

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

impl<const LOG_SIZE: usize> Default for SolverWithTable<LOG_SIZE> {
    fn default() -> Self {
        Self::new()
    }
}

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

const fn winning_score(position: &Position) -> i32 {
    ((BOARD_SIZE + 1 - position.move_count()) as i32) / 2
}

fn encode_upper_bound(score: i32) -> u8 {
    (score - MIN_SCORE + 1) as u8
}

fn encode_lower_bound(score: i32) -> u8 {
    (score + MAX_SCORE - 2 * MIN_SCORE + 2) as u8
}

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
