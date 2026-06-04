# Rust For Python Readers

This note explains the Rust features that appear in `exact_connect4`. It is written for
readers who are comfortable with Python and want enough Rust to read the solver
source notes without detouring into a full Rust textbook.

The emphasis is concrete: each section names the kind of code in this repo that
uses the idea.

## Crates, Modules, And Binaries

A Rust package can contain a library crate and one or more binary crates. In
this repo:

- `src/lib.rs` is the library root.
- `src/position.rs`, `src/solver.rs`, and the other files under `src/` are
  library modules.
- `src/bin/exact_connect4_solver.rs`, `src/bin/generator.rs`, and
  `src/bin/softmax_match.rs` are executables.

The library root declares public modules:

```rust
pub mod position;
pub mod solver;
```

This is a little like an `__init__.py` deciding which submodules belong to a
Python package, except Rust checks the module graph at compile time.

Client code refers to public items through module paths:

```rust
use exact_connect4::position::Position;
use exact_connect4::solver::Solver;
```

The `::` operator means "look inside this module, trait, or type."

## Static Types

Python usually discovers type mistakes while running. Rust checks them before
the program runs.

For example, this signature from `Position` says exactly what comes in and what
can come out:

```rust
pub fn from_sequence(sequence: &str) -> Result<Self, PlaySequenceError>
```

It takes a borrowed string slice, and it returns either a new `Position` or a
`PlaySequenceError`.

Important integer types in this repo include:

- `usize`: the natural unsigned type for indexes and lengths.
- `u8`, `u16`, `u32`, `u64`: fixed-width unsigned integers.
- `i32`: a fixed-width signed integer used for solver scores.
- `f64`: a floating-point number used for softmax probabilities.

`src/position.rs` uses this alias:

```rust
pub type Bitboard = u64;
```

That does not create a new runtime type. It gives a solver-specific name to
`u64`, making bitboard code easier to read.

## Values, References, And Mutation

Python variables are names bound to objects. Rust distinguishes owned values
from borrowed references.

This method reads a `Position` without taking ownership:

```rust
pub fn solve(&mut self, position: &Position, weak: bool) -> i32
```

The `&Position` means "borrow a position immutably." The solver can read it but
does not own it.

This method mutates an existing position:

```rust
pub fn play_bitboard(&mut self, move_bit: Bitboard)
```

The `&mut self` means "borrow this object mutably and exclusively." While that
method runs, no other code can also mutate the same value.

The solver's recursive search often copies positions by value:

```rust
let mut next_position = position;
next_position.play_bitboard(next_move);
```

That is cheap because `Position` is a small `Copy` type containing only integer
fields. This is one of the places where Rust's explicit value/reference split
maps directly to a performance choice.

## Ownership Without Garbage Collection

Rust does not use a tracing garbage collector. Values are dropped when their
owner goes out of scope.

Most short-lived values in `exact_connect4` are stack values, such as `Position`,
`MoveSorter`, and small arrays. Heap storage appears where the size is dynamic
or large:

- `Vec<K>` and `Vec<V>` in `TranspositionTable`.
- `Vec<ScoredMove>` in `SoftmaxBot`.
- `String` and `PathBuf` in command-line code.

The transposition table owns its vectors:

```rust
pub struct TranspositionTable<K, V, const LOG_SIZE: usize>
where
    K: PartialKey,
    V: Copy + Default + Eq,
{
    keys: Vec<K>,
    values: Vec<V>,
}
```

When the table is dropped, the vectors release their memory automatically.

## `struct`

A Rust `struct` groups named fields with fixed types:

```rust
pub struct Position {
    current_position: Bitboard,
    mask: Bitboard,
    moves: usize,
}
```

This is closer to a Python dataclass than to a dictionary. The field names and
types are checked by the compiler.

Fields are private unless marked `pub`. `Position` itself is public, but its
fields are private so callers must preserve the bitboard invariants through the
methods in `src/position.rs`.

Public structs can also deliberately expose fields. `ScoredMove` does this
because it is a simple result object:

```rust
pub struct ScoredMove {
    pub column: usize,
    pub score: i32,
    pub probability: f64,
}
```

## `enum`

Rust enums represent one of several named cases. They are used heavily for
structured errors:

```rust
pub enum PlayColumnError {
    InvalidColumn { column: usize },
    FullColumn { column: usize },
}
```

Each variant can carry data. This is more precise than returning a string or an
integer error code.

`BookTable` in `src/opening_book.rs` uses an enum for another purpose: choosing
one of several table representations at runtime:

```rust
enum BookTable {
    U8 { keys: Vec<u8>, values: Vec<u8> },
    U16 { keys: Vec<u16>, values: Vec<u8> },
    U32 { keys: Vec<u32>, values: Vec<u8> },
}
```

That lets the opening-book loader handle one-, two-, and four-byte partial keys
without unsafe pointer casts.

## `impl` Blocks And Methods

Methods live in `impl` blocks:

```rust
impl Position {
    pub const fn new() -> Self {
        Self {
            current_position: 0,
            mask: 0,
            moves: 0,
        }
    }
}
```

Inside `impl Position`, `Self` means `Position`.

Rust does not put methods inside the struct definition itself. The fields are
declared in one place, and methods are grouped in one or more `impl` blocks.

## `Result` Instead Of Exceptions

Rust usually represents recoverable failure with `Result<T, E>`:

```rust
pub fn play_col(&mut self, column: usize) -> Result<(), PlayColumnError>
```

This returns `Ok(())` on success or `Err(PlayColumnError)` on failure. `()` is
Rust's unit value, similar to Python's `None` when no meaningful value is
returned.

The `?` operator propagates errors:

```rust
position.play_sequence(sequence)?;
```

If `play_sequence` returns an error, the surrounding function returns that
error immediately. If it succeeds, execution continues.

The opening-book code uses the same pattern for IO and parsing:

```rust
let bytes = fs::read(path).map_err(OpeningBookError::Io)?;
Self::from_bytes(&bytes)
```

## `Option` Instead Of `None`-Capable Values

Rust uses `Option<T>` when a value may be absent:

```rust
pub struct OpeningBook {
    depth: Option<usize>,
    table: Option<BookTable>,
}
```

An `Option<T>` is either `Some(value)` or `None`.

The opening book uses `let Some(...) = ... else` to return early on missing
data:

```rust
let Some(depth) = self.depth else {
    return 0;
};
```

This is like:

```python
if self.depth is None:
    return 0
depth = self.depth
```

but the absence is explicit in the type.

## Pattern Matching

`match` handles enum variants and other patterns:

```rust
match self {
    Self::U8 { keys, values } => { /* ... */ }
    Self::U16 { keys, values } => { /* ... */ }
    Self::U32 { keys, values } => { /* ... */ }
}
```

The compiler checks that all cases are handled. That is useful for code like
`OpeningBookError`, where every error variant should have a human-readable
message.

Shorter pattern forms appear too:

```rust
if let Err(error) = solver.load_book_from_file(&options.opening_book) {
    eprintln!("Unable to load opening book: {error}");
}
```

This means "run the block only for the `Err` case."

## Traits

A trait is a set of behavior a type can implement. It is similar to a Python
protocol, but checked statically.

The transposition table defines a small trait for key fragments:

```rust
pub trait PartialKey: Copy + Default + Eq {
    fn from_key(key: u64) -> Self;
}
```

Then integer types implement it:

```rust
impl PartialKey for u16 {
    fn from_key(key: u64) -> Self {
        key as Self
    }
}
```

The generic table can then require:

```rust
K: PartialKey,
V: Copy + Default + Eq,
```

That means the table only accepts key and value types with the operations it
needs.

## Derived Traits

`#[derive(...)]` asks Rust to generate common trait implementations:

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Position {
    /* fields */
}
```

In this repo:

- `Debug` supports developer-oriented formatting.
- `Clone` supports explicit duplication.
- `Copy` means a value can be copied by simple assignment.
- `Default` supplies a default value.
- `PartialEq` and `Eq` support equality tests.

Derives are common in tests, error types, small result structs, and fixed-size
solver state.

## Generics And Const Generics

Rust generics are checked at compile time. The transposition table is generic
over key type, value type, and table size:

```rust
pub struct TranspositionTable<K, V, const LOG_SIZE: usize>
where
    K: PartialKey,
    V: Copy + Default + Eq,
{
    keys: Vec<K>,
    values: Vec<V>,
}
```

`K` and `V` are type parameters. `const LOG_SIZE: usize` is a compile-time
integer parameter.

The solver uses this to define a default table type:

```rust
type SolverTable<const LOG_SIZE: usize> = TranspositionTable<u32, u8, LOG_SIZE>;
```

This keeps the code flexible for tests while still compiling to concrete table
types.

## Arrays, Slices, And Vectors

Rust has several sequence-like types:

- `[T; N]`: fixed-size array.
- `&[T]`: borrowed slice.
- `Vec<T>`: growable heap-allocated vector.

`MoveSorter` uses a fixed array because a Connect4 position has at most seven
legal moves:

```rust
entries: [Entry; WIDTH],
```

`OpeningBook::from_bytes` takes a borrowed byte slice:

```rust
pub fn from_bytes(bytes: &[u8]) -> Result<Self, OpeningBookError>
```

`SoftmaxBot` returns a vector because the number of playable moves varies:

```rust
pub fn evaluate_moves(&mut self, position: &Position) -> Vec<ScoredMove>
```

## Iterators

Rust iterators are lazy chains of operations over sequences. They are similar
to Python generator pipelines, but statically typed.

The softmax code uses iterators to transform scored moves into weights:

```rust
let weights = moves
    .iter()
    .map(|scored_move| ((scored_move.score - max_score) as f64 / temperature).exp())
    .collect::<Vec<_>>();
```

`.iter()` borrows each item, `.map(...)` transforms each item, and
`.collect::<Vec<_>>()` builds a vector. The `_` asks the compiler to infer the
element type.

## Closures

Closures are inline functions. They use vertical bars for parameters:

```rust
.map(|score| score.exp())
```

This repo uses closures mostly in iterator chains and error conversion:

```rust
.ok_or_else(|| "missing position sequence".to_string())?;
```

`ok_or_else` receives a closure so the error string is only allocated if the
`Option` is actually `None`.

## Strings And Paths

Rust distinguishes borrowed string slices from owned strings:

- `&str`: borrowed string data.
- `String`: owned growable string.

Sequence parsing uses `&str` because it only needs to read input. The generator
creates `String` values while building move sequences.

For filesystem paths, Rust uses `Path` and `PathBuf`:

- `&Path`: borrowed path.
- `PathBuf`: owned path buffer.

Command-line option structs use `PathBuf` because parsed arguments are owned by
the options value.

## Macros

Macros end in `!`. They are expanded by the compiler before normal type
checking.

Common macros in this repo include:

```rust
println!("{sequence}");
eprintln!("Line {}: {error}", line_number + 1);
assert_eq!(solver.solve(&position, false), expected_score);
include_bytes!("../data/books/7x6.book");
```

`include_bytes!` embeds the opening book into the compiled crate so the default
solver can load it without asking the caller for a path.

## Attributes

Attributes start with `#[]` and attach metadata to the item that follows.

Examples in this repo include:

```rust
#![forbid(unsafe_code)]
#[must_use]
#[cfg(test)]
#[derive(Debug, Clone)]
```

`#![forbid(unsafe_code)]` applies to an entire crate or file. `#[must_use]`
warns if a caller ignores an important return value. `#[cfg(test)]` includes
test-only code only when running tests.

The command-line binaries also use `clap` attributes:

```rust
#[derive(Debug, Parser)]
#[command(name = "exact_connect4_solver", about = "Exact Connect4 solver")]
```

Those attributes generate command-line parsing code from the `Options` struct.

## Tests

Rust tests usually live near the code they test or in `tests/` integration
tests.

Unit tests inside source files use:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn detects_immediate_wins() {
        /* ... */
    }
}
```

Integration tests in `tests/` compile as separate crates and use the public API
or run binaries. This repo uses both styles:

- `src/position.rs` tests private bitboard details.
- `tests/exact_connect4_solver_cli.rs` tests command-line behavior.
- `tests/fixture_files.rs` checks tracked fixture files.

## Reading This Repo

For algorithm code, start with:

- `src/position.rs` for bitboards and legal move generation.
- `src/move_sorter.rs` for fixed-size move ordering.
- `src/transposition_table.rs` for cached bounds.
- `src/solver.rs` for negamax alpha-beta search.
- `src/opening_book.rs` for compatible book loading.

For Rust mechanics, watch for the recurring pattern:

1. Public APIs use explicit types and `Result`/`Option`.
2. Hot solver state is small and copyable.
3. Large or variable-size storage is owned by `Vec`.
4. Command-line code uses higher-level owned types such as `String` and
   `PathBuf`.
