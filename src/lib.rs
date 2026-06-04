// SPDX-FileCopyrightText: 2017-2019 Pascal Pons <contact@gamesolver.org>
// SPDX-FileCopyrightText: 2026 Jonathan Yedidia
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

pub mod opening_book;
pub mod position;
pub mod softmax_bot;
pub mod solver;
#[doc(hidden)]
pub mod transposition_table;

mod move_sorter;

pub use opening_book::{OpeningBook, OpeningBookError};
pub use position::{PlayColumnError, PlaySequenceError, PlaySequenceErrorKind, Position};
pub use softmax_bot::{ScoredMove, SoftmaxBot, SoftmaxBotConfig, SoftmaxMoveSelection};
pub use solver::{INVALID_MOVE, Solver, SolverWithTable};

/// Path of the tracked 7x6 opening book inside this repository.
pub const DEFAULT_OPENING_BOOK_PATH: &str = "data/books/7x6.book";

/// Bytes of the tracked 7x6 opening book.
///
/// The default solver parses these bytes at construction time so ordinary
/// library users get opening-book acceleration without knowing this path.
pub const DEFAULT_OPENING_BOOK_BYTES: &[u8] = include_bytes!("../data/books/7x6.book");

/// Returns the bytes of the tracked default opening book.
#[must_use]
pub fn default_opening_book_bytes() -> &'static [u8] {
    DEFAULT_OPENING_BOOK_BYTES
}

#[cfg(test)]
mod tests {
    use super::{DEFAULT_OPENING_BOOK_PATH, default_opening_book_bytes};

    #[test]
    fn default_opening_book_is_embedded() {
        assert_eq!(DEFAULT_OPENING_BOOK_PATH, "data/books/7x6.book");
        assert!(!default_opening_book_bytes().is_empty());
    }
}
