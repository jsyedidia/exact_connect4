# Command-Line Solver

The `exact_connect4_solver` binary reads Connect4 positions from standard
input. Each line is a sequence of 1-based column numbers.

## Build

```sh
cargo build --release --bin exact_connect4_solver
```

## Solve Positions

```sh
echo 32164625 | cargo solver
```

Output has the input sequence followed by the perfect-play score:

```text
32164625 11
```

You can also pipe a file of sequences:

```sh
cargo solver < tests/fixtures/positions/Seq_L1_R1
```

## Analyze Every Column

Use `-a` to print one score per column:

```sh
echo 177322644317353514472267227353611516544566 | cargo solver -a
```

The first field is the input sequence. The next seven fields are scores for
columns 1 through 7. Illegal columns are reported as `-1000`.

## Weak Solving

Use `-w` when only the win/draw/loss result matters:

```sh
echo 32164625 | cargo solver -w
```

Weak scores are in `-1..=1`.

## Opening Books

By default, the solver uses the tracked `data/books/7x6.book` opening book
embedded in the crate. To load a compatible book file from disk:

```sh
echo 32164625 | cargo solver -b data/books/7x6.book
```

If the requested book cannot be loaded, the binary prints an error to stderr and
continues without a book.

## Benchmark Check

The benchmark fixtures contain `<sequence> <score>` lines. To check one file:

```sh
tmp_output="$(mktemp)"
awk '{ print $1 }' tests/fixtures/benchmarks/Test_L1_R1 \
  | target/release/exact_connect4_solver > "$tmp_output"
diff -u tests/fixtures/benchmarks/Test_L1_R1 "$tmp_output"
rm "$tmp_output"
```
