# exact_connect4

`exact_connect4` is an exact Connect4 solver in Rust. It provides a library for
parsing positions and solving them under perfect play, plus command-line tools
for solving positions, analyzing moves, generating book inputs, and running
matches between perfect and probabilistically imperfect bots.

The solver is a Rust port of Pascal Pons' C++ exact Connect4 solver (available
at <https://github.com/pascalpons/connect4>) and uses the
tracked `data/books/7x6.book` opening book by default. My benchmarking tests
find that it is roughly the same speed as the original C++ solver.

## Layout

- `src/`: library implementation
- `src/bin/`: command-line tools
- `tests/`: regression tests and fixture checks
- `tests/fixtures/`: benchmark, position, and manual test data
- `data/books/`: tracked opening books
- `docs/`: command-line and benchmark guides
- `notes/`: detailed explanations of the code and algorithm

## Build And Test

```sh
cargo build --release
cargo test --release
```

Release builds use link-time optimization because the solver's hot path benefits
measurably from whole-program optimization.

## Command Line

This repo defines Cargo aliases in `.cargo/config.toml` for the release-mode
tools. For example, `cargo solver -a` expands to
`cargo run --release --bin exact_connect4_solver -- -a`.

Get help for the solver:

```sh
cargo solver --help
```

Solve one position:

```sh
echo 32164625 | cargo solver
```

Analyze every column:

```sh
echo 32164625 | cargo solver -a
```

Get help for the softmax match tool:

```sh
cargo softmax_match --help
```

Generate unique non-terminal sequences through depth 2:

```sh
cargo generator 2
```

See `docs/command_line_solver.md` and `docs/benchmarking.md` for more examples.

## Library Example

```rust,no_run
use exact_connect4::{Position, Solver};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let position = Position::from_sequence("32164625")?;
let mut solver = Solver::new();

assert_eq!(solver.solve(&position, false), 11);
# Ok(())
# }
```

## Documentation

- `docs/` contains guides to using the solver and benchmarks.
- `notes/` contains detailed explanations of the algorithms and source code. See [notes/README.md](notes/README.md) for an overview of the notes and how to navigate them.
