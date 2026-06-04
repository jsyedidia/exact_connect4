# Notes

This directory contains prose explanations of the `exact_connect4` codebase, a
Rust exact solver for Connect4. The notes are meant to make the implementation
readable: they explain the game concepts, the search algorithm, the Rust design
choices, and the source files that carry those ideas.

## Who These Notes Are For

The primary audience is a programmer or researcher who wants to understand how
the exact solver works and how this repository implements it. You do not need
deep Rust experience. Two background notes explain the Rust features used here
for readers coming from different languages:

1. **[concepts/rust_for_python_readers.md](concepts/rust_for_python_readers.md)** - Rust ideas for readers more
   comfortable in Python.
2. **[concepts/rust_for_cpp_readers.md](concepts/rust_for_cpp_readers.md)** - Rust ideas for readers more
   comfortable in C++.

Readers who already know Rust can skip those and start with the Connect4
concept notes.

## Directory Structure

The notes are organized into two categories:

- **`concepts/`** - Standalone explanations of ideas that span multiple source
  files. These cover Connect4 rules, scores, bitboards, alpha-beta search,
  move ordering, transposition tables, opening books, softmax bots, and Rust
  idioms.
- **`src/`** - File-by-file walkthroughs that mirror the source tree. Each
  source note covers one Rust file in source order.

Source notes are named after the file they document. For example:

- [notes/src/position.rs.md](src/position.rs.md) documents `src/position.rs`.
- [notes/src/bin/exact_connect4_solver.rs.md](src/bin/exact_connect4_solver.rs.md) documents
  `src/bin/exact_connect4_solver.rs`.

User-facing command-line and benchmark guides live outside this tree in
`docs/`.

## Suggested Reading Order

The notes are designed to be read roughly in this order. You can skip ahead if
a topic is already familiar.

### 1. Rust Background

Read one of these only if Rust is still new or if a source note refers to a
language feature you want explained.

1. **[concepts/rust_for_python_readers.md](concepts/rust_for_python_readers.md)** - Modules, ownership, borrowing,
   `Result`, `Option`, traits, generics, iterators, tests, and command-line
   code from a Python reader's point of view.
2. **[concepts/rust_for_cpp_readers.md](concepts/rust_for_cpp_readers.md)** - Crates, ownership, `Copy`,
   `Option`, `Result`, enums, traits, const generics, slices, vectors, and
   safe Rust from a C++ reader's point of view.

### 2. Game And Representation

Start here to understand what a position means and how the board is encoded.

3. **[concepts/connect4_rules.md](concepts/connect4_rules.md)** - Board rules, move-sequence notation,
   valid solver positions, and fixture files.
4. **[concepts/solver_scores.md](concepts/solver_scores.md)** - Exact win/loss/draw scores, benchmark
   score format, and invalid move markers.
5. **[concepts/bitboards.md](concepts/bitboards.md)** - The 7x6 sentinel-bit layout, masks, keys, and
   winning-cell detection.
6. **[src/lib.rs.md](src/lib.rs.md)** - The crate root, public modules, re-exports, and
   embedded default book.
7. **[src/position.rs.md](src/position.rs.md)** - The source walkthrough for board state, parsing,
   move generation, win detection, and keys.

### 3. Search Helpers

These notes cover the small performance-critical pieces used by the solver.

8. **[concepts/move_ordering.md](concepts/move_ordering.md)** - Why move ordering matters for alpha-beta
   search and how moves are scored.
9. **[src/move_sorter.rs.md](src/move_sorter.rs.md)** - The fixed-capacity insertion sorter for at
   most seven Connect4 moves.
10. **[concepts/transposition_tables.md](concepts/transposition_tables.md)** - Cache entries, partial keys,
   collisions, missing values, and table sizing.
11. **[src/transposition_table.rs.md](src/transposition_table.rs.md)** - The generic fixed-size table used by
    the solver and book tooling.

### 4. Exact Solver

These notes explain the recursive search itself.

12. **[concepts/alpha_beta.md](concepts/alpha_beta.md)** - Score windows, cutoffs, bounds, and why
    ordering affects speed.
13. **[concepts/negamax.md](concepts/negamax.md)** - The side-to-move perspective and the recursive
    score negation.
14. **[src/solver.rs.md](src/solver.rs.md)** - The exact solver, including null-window search,
    non-losing moves, transposition-table bounds, move ordering, opening-book
    lookup, and analysis mode.

### 5. Opening Book And Binaries

These notes explain how the solver gets practical early-game speed and how the
command-line tools are wired.

15. **[concepts/opening_books.md](concepts/opening_books.md)** - The binary book format, `key3`, score
    offsets, lookup behavior, and integration point in the solver.
16. **[src/opening_book.rs.md](src/opening_book.rs.md)** - Header validation, decoded key storage,
    lookup, and opening-book errors.
17. **[src/bin/exact_connect4_solver.rs.md](src/bin/exact_connect4_solver.rs.md)** - The command-line solver frontend.

The external user guides are:

18. **[../docs/command_line_solver.md](../docs/command_line_solver.md)** - Command-line examples.
19. **[../docs/benchmarking.md](../docs/benchmarking.md)** - Fixture checks and benchmark-file usage.

### 6. Gameplay Utilities And Tools

These notes cover code built on top of the exact solver.

20. **[concepts/softmax_bots.md](concepts/softmax_bots.md)** - How exact solver scores become greedy or
    probabilistic move choices.
21. **[src/softmax_bot.rs.md](src/softmax_bot.rs.md)** - Turning exact move scores into greedy or
    softmax probabilities.
22. **[src/bin/softmax_match.rs.md](src/bin/softmax_match.rs.md)** - Running matches between two softmax
    bots.
23. **[src/bin/generator.rs.md](src/bin/generator.rs.md)** - Generating unique sequence inputs or a
    compatible book file from scored positions.

## Tips For Reading

- **Read the concept notes before the matching source notes.** For example,
  [concepts/bitboards.md](concepts/bitboards.md) gives the mental model that
  makes [src/position.rs.md](src/position.rs.md) easier to follow.

- **Follow cross-references.** Source notes include "Related concept notes"
  near the top. Those links point to the broader explanation behind the file.

- **Use the source notes as annotated source.** The central source notes include
  every non-comment Rust code line in source order. You can read them without
  keeping the `.rs` file open, although having both side by side can help.

- **Expect private helpers to matter.** Much of the solver's behavior lives in
  small private functions such as mask builders, score encoders, and table
  lookup helpers. Source notes explain those because they are part of the real
  implementation, even when they are not public API.

- **Treat benchmark numbers as local measurements.** Concept notes explain
  stable performance ideas. Machine-dependent timing logs should not be read as
  universal claims.

## Related Project Documents

- [../README.md](../README.md) - Project overview.
- [../docs/command_line_solver.md](../docs/command_line_solver.md) - Command-line usage.
- [../docs/benchmarking.md](../docs/benchmarking.md) - Fixture and benchmark checks.
