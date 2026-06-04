# src/lib.rs

## Role In The System

`src/lib.rs` is the library crate root. It decides which modules are part of
the crate, re-exports the stable public entry points, embeds the default
opening book, and includes the top-level README as crate documentation.

Related concept notes:

- [notes/concepts/rust_for_python_readers.md](../concepts/rust_for_python_readers.md)
- [notes/concepts/rust_for_cpp_readers.md](../concepts/rust_for_cpp_readers.md)
- [notes/concepts/opening_books.md](../concepts/opening_books.md)

## Code Walkthrough

### Crate Attributes

```rust
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]
```

The crate forbids unsafe code. The second attribute uses the repository README
as crate-level Rustdoc, so the README examples are checked as doctests.

### Public Modules

```rust
pub mod opening_book;
pub mod position;
pub mod softmax_bot;
pub mod solver;
#[doc(hidden)]
pub mod transposition_table;
```

The public modules expose the stable user-facing solver pieces. The
transposition-table module remains public but hidden from generated docs because
the `generator` binary uses its table-sizing helper; ordinary library users
should not need it.

### Internal Modules

```rust
mod move_sorter;
```

`move_sorter` is an implementation detail of the solver. Keeping the module
private reduces the public API surface while still letting `solver.rs` use it
inside the crate.

### Public Re-Exports

```rust
pub use opening_book::{OpeningBook, OpeningBookError};
pub use position::{PlayColumnError, PlaySequenceError, PlaySequenceErrorKind, Position};
pub use softmax_bot::{ScoredMove, SoftmaxBot, SoftmaxBotConfig, SoftmaxMoveSelection};
pub use solver::{INVALID_MOVE, Solver, SolverWithTable};
```

These re-exports make the main API available from the crate root. Callers can
write `exact_connect4::Position` and `exact_connect4::Solver` while the source files remain organized
by module.

### Default Opening Book

```rust
pub const DEFAULT_OPENING_BOOK_PATH: &str = "data/books/7x6.book";

pub const DEFAULT_OPENING_BOOK_BYTES: &[u8] = include_bytes!("../data/books/7x6.book");

#[must_use]
pub fn default_opening_book_bytes() -> &'static [u8] {
    DEFAULT_OPENING_BOOK_BYTES
}
```

`DEFAULT_OPENING_BOOK_PATH` names the tracked book file for command-line
defaults and user-facing messages. `DEFAULT_OPENING_BOOK_BYTES` embeds that
file into the crate. `default_opening_book_bytes` exposes the embedded bytes to
the opening-book loader.

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::{DEFAULT_OPENING_BOOK_PATH, default_opening_book_bytes};

    #[test]
    fn default_opening_book_is_embedded() {
        assert_eq!(DEFAULT_OPENING_BOOK_PATH, "data/books/7x6.book");
        assert!(!default_opening_book_bytes().is_empty());
    }
}
```

The unit test protects the public default-book path and verifies that the
embedded book data is present.

## Important Invariants

- The default opening book must stay available from a clean checkout.
- The main public API should stay small: positions, solving, opening books, and
  softmax play are public; search helpers should remain internal or hidden.
- The README is crate documentation, so README Rust examples must keep
  compiling.

## Extension Notes

Add new modules here only when they are intended to be part of the crate
boundary. For internal helpers, prefer private modules and expose behavior
through the higher-level solver or tool APIs.
