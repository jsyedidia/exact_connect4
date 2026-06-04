# Benchmarking

This repo keeps benchmark fixtures in `tests/fixtures/benchmarks`. Each line is:

```text
<move-sequence> <expected-score>
```

The solver binaries read sequence-only input, so benchmark checks pipe the first
column into the solver and compare the produced output with the original file.

## Build Release Binaries

```sh
cargo build --release --bins
```

## Check One Benchmark File

```sh
tmp_output="$(mktemp)"
awk '{ print $1 }' tests/fixtures/benchmarks/Test_L1_R1 \
  | target/release/exact_connect4_solver > "$tmp_output"
diff -u tests/fixtures/benchmarks/Test_L1_R1 "$tmp_output"
rm "$tmp_output"
```

## Check All Benchmark Files

```sh
tmpdir="$(mktemp -d)"
for benchmark_file in tests/fixtures/benchmarks/*; do
  output_file="$tmpdir/$(basename "$benchmark_file")"
  awk '{ print $1 }' "$benchmark_file" \
    | target/release/exact_connect4_solver > "$output_file"
  diff -u "$benchmark_file" "$output_file"
done
rm -rf "$tmpdir"
```

## Generate Sequence Inputs

The `generator` tool can print unique non-terminal move sequences through a
requested depth:

```sh
cargo generator 2
```

Depth `0` prints the empty starting sequence. Depth `1` prints the empty
sequence and the four unique first moves after mirror deduplication.

## Generate A Book File

Without a depth argument, `generator` reads benchmark-style `<sequence> <score>`
lines from stdin and writes `7x6.book` in the current directory:

```sh
awk '{ print $1, $2 }' tests/fixtures/benchmarks/Test_L1_R1 \
  | cargo generator
```

The root-level generated `7x6.book` is ignored by Git. The tracked default book
remains `data/books/7x6.book`.
