# Opening Books

An opening book is a precomputed table of known positions near the start of the
game. Exact Connect4 search is most expensive from early positions because the
tree is widest there. A book lets the solver skip that early search when a
stored position is available.

The tracked default book is `data/books/7x6.book`. It is embedded into the Rust
crate with `include_bytes!`, so `Solver::new()` can load it without asking the
caller for a path.

## Binary Format

The file begins with a six-byte header:

```text
byte 0: board width
byte 1: board height
byte 2: maximum stored move depth
byte 3: partial-key size in bytes
byte 4: value size in bytes
byte 5: log table size
```

After the header, the file stores all partial keys first, then all values. The
table capacity is the first prime greater than or equal to `2^log_size`.

For the tracked `7x6.book`, the header is:

```text
7 6 14 1 1 24
```

That means a 7x6 board, entries through move depth 14, one byte per partial key,
one byte per value, and a prime capacity near `2^24`.

## Endianness

The original book format stores integer key arrays as raw bytes. For one-byte
keys this is independent of endianness. For two-byte and four-byte keys, this
Rust loader decodes keys as little-endian integers because the existing books
used by this project were produced in that layout.

## Lookup

The book is queried with `Position::key3()`, which canonicalizes mirrored
positions. A lookup returns zero when:

- no book is loaded,
- the position is deeper than the book depth,
- the table slot contains a different partial key,
- the stored value is zero.

Nonzero values use the same score offset as the transposition table's upper
bound range:

```text
score = raw_value + MIN_SCORE - 1
```

The exact solver checks the book after transposition-table bounds have narrowed
the alpha-beta window and before child moves are generated.

## Implemented In This Repo

Opening-book support is implemented in `src/opening_book.rs` and integrated by
`src/solver.rs`.

The main code-level pieces are:

- `DEFAULT_OPENING_BOOK_BYTES` in `src/lib.rs` embeds `data/books/7x6.book`
  with `include_bytes!`.
- `OpeningBook::from_default_book`, `OpeningBook::from_file`, and
  `OpeningBook::from_bytes` construct a book from the embedded file, a path, or
  raw bytes.
- `OpeningBook::from_bytes` validates the six-byte header and splits the data
  into key and value regions.
- `BookTable` stores the decoded key array as `Vec<u8>`, `Vec<u16>`, or
  `Vec<u32>`, depending on the header.
- `BookTable::get` performs the slot lookup and partial-key check.
- `OpeningBook::get` rejects unloaded books, positions deeper than the book,
  and missing table entries by returning zero.
- `Solver::new` loads the embedded default book.
- `SolverWithTable::negamax` queries the book before generating child moves.
- `src/bin/exact_connect4_solver.rs` accepts `-b FILE` for a compatible book override.
- `src/bin/generator.rs` can write a compatible book file from scored
  position lines.

The matching source notes are
[notes/src/opening_book.rs.md](../src/opening_book.rs.md),
[notes/src/solver.rs.md](../src/solver.rs.md),
[notes/src/bin/exact_connect4_solver.rs.md](../src/bin/exact_connect4_solver.rs.md), and
[notes/src/bin/generator.rs.md](../src/bin/generator.rs.md).
