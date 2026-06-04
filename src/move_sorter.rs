// SPDX-FileCopyrightText: 2017-2019 Pascal Pons <contact@gamesolver.org>
// SPDX-FileCopyrightText: 2026 Jonathan Yedidia
// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::position::{Bitboard, WIDTH};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Entry {
    move_bit: Bitboard,
    score: i32,
}

/// Fixed-capacity move sorter for the seven legal columns of a Connect4 board.
#[derive(Debug, Clone)]
pub struct MoveSorter {
    entries: [Entry; WIDTH],
    len: usize,
}

impl MoveSorter {
    /// Creates an empty move sorter.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: [Entry {
                move_bit: 0,
                score: 0,
            }; WIDTH],
            len: 0,
        }
    }

    /// Adds a move with its ordering score.
    ///
    /// Moves are kept in increasing score order internally, so repeated calls
    /// to `pop` return the highest-scoring remaining move.
    pub fn add(&mut self, move_bit: Bitboard, score: i32) {
        assert!(self.len < WIDTH, "move sorter capacity exceeded");

        let mut position = self.len;
        self.len += 1;

        while position > 0 && self.entries[position - 1].score > score {
            self.entries[position] = self.entries[position - 1];
            position -= 1;
        }

        self.entries[position] = Entry { move_bit, score };
    }

    /// Removes and returns the highest-scoring move.
    pub fn pop(&mut self) -> Option<Bitboard> {
        if self.len == 0 {
            return None;
        }

        self.len -= 1;
        Some(self.entries[self.len].move_bit)
    }
}

impl Default for MoveSorter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::MoveSorter;
    use crate::position::WIDTH;

    #[test]
    fn pops_moves_from_highest_score_to_lowest_score() {
        let mut sorter = MoveSorter::new();

        sorter.add(10, 4);
        sorter.add(20, 1);
        sorter.add(30, 7);
        sorter.add(40, 3);

        assert_eq!(sorter.pop(), Some(30));
        assert_eq!(sorter.pop(), Some(10));
        assert_eq!(sorter.pop(), Some(40));
        assert_eq!(sorter.pop(), Some(20));
        assert_eq!(sorter.pop(), None);
    }

    #[test]
    fn equal_scores_pop_newest_first() {
        let mut sorter = MoveSorter::new();

        sorter.add(10, 5);
        sorter.add(20, 5);
        sorter.add(30, 5);

        assert_eq!(sorter.pop(), Some(30));
        assert_eq!(sorter.pop(), Some(20));
        assert_eq!(sorter.pop(), Some(10));
    }

    #[test]
    #[should_panic(expected = "move sorter capacity exceeded")]
    fn panics_when_more_than_width_moves_are_added() {
        let mut sorter = MoveSorter::new();

        for index in 0..=WIDTH {
            sorter.add(index as u64 + 1, index as i32);
        }
    }
}
