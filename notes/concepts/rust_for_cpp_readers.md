# Rust For C++ Readers

This note explains the Rust features that matter for reading `exact_connect4`, assuming the
reader is already comfortable with C++ concepts such as values, references,
templates, RAII, const-correctness, and standard containers.

The goal is not to teach all of Rust. The goal is to make this repository's
Rust implementation readable.

## Crates Instead Of Translation Units

Rust code is organized around crates and modules rather than header/source
translation units.

In this repo:

- `src/lib.rs` is the library crate root.
- `src/position.rs`, `src/solver.rs`, and similar files are modules.
- `src/bin/*.rs` files are binary crates.

The library root declares modules explicitly:

```rust
pub mod position;
pub mod solver;
```

There is no separate header file. Public items, private items, and
implementations live in the Rust source module. The compiler builds the whole
crate as a unit, so Rust does not need textual includes or forward declarations
for normal module structure.

## Visibility Is Private By Default

Rust items are private to their module unless marked `pub`.

```rust
pub struct Position {
    current_position: Bitboard,
    mask: Bitboard,
    moves: usize,
}
```

This makes the `Position` type public while keeping its fields private. That is
similar to a C++ class with public methods and private data members, except
Rust uses the same `struct` keyword for both public-field and private-field
types.

Fields can be public when the type is intentionally a simple data result:

```rust
pub struct ScoredMove {
    pub column: usize,
    pub score: i32,
    pub probability: f64,
}
```

## RAII Without Constructors And Destructors Everywhere

Rust uses deterministic destruction like C++. Values are dropped when they go
out of scope, and owned heap allocations release themselves automatically.

The table owns two vectors:

```rust
keys: Vec<K>,
values: Vec<V>,
```

When `TranspositionTable` is dropped, those vectors are dropped too.

Rust does not use C++ constructors in the same way. It usually uses associated
functions such as:

```rust
pub fn new() -> Self
```

There is also a `Drop` trait for custom cleanup, but this repo does not need
custom destructors. Standard library types own their resources directly.

## Ownership, Borrowing, And References

Rust references look familiar but carry stronger compile-time rules.

```rust
pub fn solve(&mut self, position: &Position, weak: bool) -> i32
```

`&Position` is a shared borrow, roughly comparable to `const Position&`.
`&mut self` is an exclusive mutable borrow, comparable to a non-const reference
with an additional aliasing guarantee: while the mutable borrow exists, no other
borrow may access the same value.

That exclusivity is central to Rust's memory model. It lets safe Rust avoid
data races and many iterator-invalidation-style bugs without runtime checks.

## Moves And `Copy`

Rust assignments move values by default. After a non-`Copy` value is moved, the
old binding cannot be used.

Types made only of simple copyable fields can implement `Copy`:

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Position {
    current_position: Bitboard,
    mask: Bitboard,
    moves: usize,
}
```

For `Position`, assignment copies the three fields. That is deliberate: the
solver recursively creates child positions by copying a small value and playing
one move.

Heap-owning types such as `Vec<T>`, `String`, and `PathBuf` are not `Copy`.
Moving them transfers ownership of the allocation. Cloning them is explicit.

## No Null References

Safe Rust references cannot be null. Optional values use `Option<T>`:

```rust
pub struct OpeningBook {
    depth: Option<usize>,
    table: Option<BookTable>,
}
```

This is comparable to `std::optional<T>`, not to a nullable pointer.

Pattern matching extracts the present value:

```rust
let Some(depth) = self.depth else {
    return 0;
};
```

The type system forces code to handle `None`.

## `Result` Instead Of Exceptions

Recoverable failure is usually represented with `Result<T, E>`:

```rust
pub fn from_file(path: impl AsRef<Path>) -> Result<Self, OpeningBookError>
```

The return value is either `Ok(Self)` or `Err(OpeningBookError)`.

The `?` operator is the usual propagation mechanism:

```rust
let bytes = fs::read(path).map_err(OpeningBookError::Io)?;
Self::from_bytes(&bytes)
```

If the read fails, the function returns the error. If it succeeds, `bytes`
receives the successful value.

The command-line binaries convert errors into stderr messages and exit codes.
The library APIs keep errors typed.

## Enums Are Tagged Unions

Rust enums are algebraic data types. They are close to a type-safe tagged union
or `std::variant`, but integrated into the language.

The opening-book table uses an enum to represent the supported key widths:

```rust
enum BookTable {
    U8 { keys: Vec<u8>, values: Vec<u8> },
    U16 { keys: Vec<u16>, values: Vec<u8> },
    U32 { keys: Vec<u32>, values: Vec<u8> },
}
```

`match` dispatches on the active variant:

```rust
match self {
    Self::U8 { keys, values } => { /* ... */ }
    Self::U16 { keys, values } => { /* ... */ }
    Self::U32 { keys, values } => { /* ... */ }
}
```

The compiler checks that every variant is handled. This is why error display
code and book-table lookup code can be exhaustive without a default branch.

## Traits Instead Of Inheritance For Shared Behavior

Rust traits define behavior that types can implement. They are closer to C++
concepts plus interfaces than to base classes.

The transposition table defines:

```rust
pub trait PartialKey: Copy + Default + Eq {
    fn from_key(key: u64) -> Self;
}
```

Then primitive integer types implement it:

```rust
impl PartialKey for u32 {
    fn from_key(key: u64) -> Self {
        key as Self
    }
}
```

The generic table constrains type parameters with trait bounds:

```rust
K: PartialKey,
V: Copy + Default + Eq,
```

This is the Rust equivalent of saying the template only accepts types that
support the operations the table needs.

This repo does not use trait objects in the hot solver path. The important
polymorphism is static.

## Generics And Const Generics

Rust generics are monomorphized much like C++ templates, but the syntax and
constraints are explicit.

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

`K` and `V` are type parameters. `const LOG_SIZE: usize` is a compile-time value
parameter, similar in spirit to a non-type template parameter.

The solver fixes the concrete key and value types:

```rust
type SolverTable<const LOG_SIZE: usize> = TranspositionTable<u32, u8, LOG_SIZE>;
```

Unit tests can instantiate smaller tables while the default solver uses the
normal production size.

## `impl Trait` Parameters

Some function parameters use `impl Trait`:

```rust
pub fn load_book_from_file(
    &mut self,
    path: impl AsRef<std::path::Path>,
) -> Result<(), OpeningBookError>
```

This means "accept any type that implements `AsRef<Path>`." It is comparable to
a constrained function template. Callers can pass `&Path`, `PathBuf`, and other
path-like values.

## Associated Functions And Methods

Rust separates data declarations from method implementations:

```rust
impl OpeningBook {
    pub const fn new() -> Self {
        Self {
            depth: None,
            table: None,
        }
    }
}
```

Functions without a `self` parameter are associated functions, called with
`OpeningBook::new()`. Functions with `&self` or `&mut self` are methods, called
with dot syntax.

Trait implementations also use `impl`:

```rust
impl Default for OpeningBook {
    fn default() -> Self {
        Self::new()
    }
}
```

## `const`, `const fn`, And Compile-Time Values

Rust `const` items are compile-time constants:

```rust
pub const WIDTH: usize = 7;
pub const HEIGHT: usize = 6;
```

A `const fn` can run in a compile-time context:

```rust
const fn bottom_mask() -> Bitboard {
    let mut mask = 0;
    let mut column = 0;
    while column < WIDTH {
        mask |= 1_u64 << (column * COLUMN_STRIDE);
        column += 1;
    }
    mask
}
```

`src/position.rs` uses this to compute masks while keeping them as constants:

```rust
pub const BOTTOM_MASK: Bitboard = bottom_mask();
```

## Arrays, Slices, And `Vec`

Rust distinguishes fixed arrays, borrowed slices, and owned vectors:

- `[T; N]`: fixed-size array.
- `&[T]`: borrowed view of contiguous elements.
- `Vec<T>`: growable owning vector.

`MoveSorter` uses a fixed array because Connect4 has at most seven legal moves:

```rust
entries: [Entry; WIDTH],
```

`OpeningBook::from_bytes` accepts a borrowed byte slice:

```rust
pub fn from_bytes(bytes: &[u8]) -> Result<Self, OpeningBookError>
```

`BookTable` owns vectors because opening-book size is determined by the book
header at runtime.

## Strings And Paths

Rust has owned and borrowed string/path types:

- `&str`: borrowed UTF-8 string slice.
- `String`: owned UTF-8 string.
- `&Path`: borrowed filesystem path.
- `PathBuf`: owned filesystem path.

`Position::from_sequence` takes `&str` because it only reads the move sequence.
Command-line option structs store `PathBuf` because parsed arguments are owned
by the options value.

## Iterators And Closures

Rust iterator chains are lazy and statically typed. The softmax code uses them
to compute weights:

```rust
let weights = moves
    .iter()
    .map(|scored_move| ((scored_move.score - max_score) as f64 / temperature).exp())
    .collect::<Vec<_>>();
```

The closure syntax is `|arg| expression`. `collect::<Vec<_>>()` specifies the
output collection while letting the compiler infer the element type.

In optimized builds, these iterator chains usually compile down to loops. The
solver's hottest recursive code still uses explicit loops where that is clearer
and easier to keep allocation-free.

## Macros And Attributes

Macros end in `!`:

```rust
println!("{sequence}");
assert_eq!(actual, expected);
include_bytes!("../data/books/7x6.book");
```

`include_bytes!` embeds the default opening book in the crate at compile time.

Attributes attach metadata or code-generation instructions:

```rust
#![forbid(unsafe_code)]
#[derive(Debug, Clone)]
#[cfg(test)]
#[must_use]
```

The command-line programs use `clap` derive macros:

```rust
#[derive(Debug, Parser)]
#[command(name = "exact_connect4_solver", about = "Exact Connect4 solver")]
```

This generates argument parsing code from the `Options` struct.

## Unsafe Is Absent Here

This repo currently starts each crate with:

```rust
#![forbid(unsafe_code)]
```

That means no unsafe Rust is allowed in these crates. The opening-book loader,
for example, uses typed vectors and enum dispatch instead of pointer casting.

This is an important design choice for a port of a performance-sensitive C++
program: the Rust code still aims for near-C++ speed, but it first relies on
safe layout choices, static dispatch, and release optimization.

## Testing Layout

Rust unit tests can live in the same file as the code being tested:

```rust
#[cfg(test)]
mod tests {
    #[test]
    fn detects_immediate_wins() {
        /* ... */
    }
}
```

That is useful for private helpers such as bit masks and bound encodings.

Integration tests live under `tests/` and compile as separate crates. This repo
uses them for fixture validation and CLI behavior.

## C++ Instincts To Adjust

When reading this repo, a few C++ instincts need translation:

- Look for modules, not headers.
- Treat `&mut` as both mutable and exclusive.
- Expect recoverable errors in return types, not exceptions.
- Use `Option<T>` and `Result<T, E>` instead of nullable values and status
  codes.
- Expect generic constraints in `where` clauses.
- Remember that moving an owning value transfers ownership unless the type is
  `Copy`.
- Do not look for manual destructors unless a type owns a non-Rust resource.

The solver is still built from familiar systems-programming pieces: fixed-size
integers, arrays, heap vectors, static dispatch, deterministic cleanup, and
release-mode optimization. Rust makes the ownership and error paths visible in
the type system.
