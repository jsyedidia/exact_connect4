# src/bin/generator.rs

## Role In The System

`src/bin/generator.rs` is a developer tool with two modes.

With a depth argument, it prints unique non-terminal move sequences through that
depth. Without arguments, it reads `<sequence> <score>` lines from stdin and
writes a compatible `7x6.book` file in the current directory.

The generated root-level `7x6.book` is ignored by Git. The tracked default book
lives at `data/books/7x6.book`.

Related concept notes:

- [notes/concepts/connect4_rules.md](../../concepts/connect4_rules.md)
- [notes/concepts/bitboards.md](../../concepts/bitboards.md)
- [notes/concepts/opening_books.md](../../concepts/opening_books.md)
- [notes/concepts/solver_scores.md](../../concepts/solver_scores.md)

## Code Walkthrough

### Crate Attribute

```rust
#![forbid(unsafe_code)]
```

The tool forbids unsafe code.

### Imports

```rust
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufWriter, Write};
use std::process::ExitCode;

use exact_connect4::position::{HEIGHT, MAX_SCORE, MIN_SCORE, Position, WIDTH};
use exact_connect4::transposition_table::next_prime;
```

The generator uses a `HashSet` to avoid revisiting symmetric positions,
standard IO for stdin/stdout and book writing, and the library position and
table-sizing helpers.

### Book Constants

```rust
const BOOK_LOG_SIZE: u8 = 23;
const BOOK_DEPTH: u8 = 14;
const BOOK_PARTIAL_KEY_BYTES: u8 = 2;
const BOOK_VALUE_BYTES: u8 = 1;
const OUTPUT_BOOK_FILE: &str = "7x6.book";
```

These constants define the book produced by stdin mode. It writes a depth-14
book with a prime table capacity near `2^23`, two-byte partial keys, and
one-byte values.

### Main Function

```rust
fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    match (args.next(), args.next()) {
        (Some(depth), None) => match depth.parse::<i32>() {
            Ok(depth) => {
                if let Err(error) = generate_sequences(depth) {
                    eprintln!("{error}");
                    ExitCode::from(1)
                } else {
                    ExitCode::SUCCESS
                }
            }
            Err(_) => {
                eprintln!("Invalid depth: {depth}");
                ExitCode::from(1)
            }
        },
        (None, None) => {
            if let Err(error) = generate_opening_book() {
                eprintln!("{error}");
                ExitCode::from(1)
            } else {
                ExitCode::SUCCESS
            }
        }
        _ => {
            eprintln!("Usage: generator [DEPTH]");
            ExitCode::from(1)
        }
    }
}
```

One argument selects sequence-generation mode. No arguments selects book-writing
mode. More than one argument is a usage error.

### Sequence Generation

```rust
fn generate_sequences(depth: i32) -> io::Result<()> {
    let stdout = io::stdout();
    let mut output = BufWriter::new(stdout.lock());
    let mut visited_positions = HashSet::new();
    let mut move_sequence = String::with_capacity(depth.max(0) as usize);

    explore(
        Position::new(),
        &mut move_sequence,
        depth,
        &mut visited_positions,
        &mut output,
    )?;
    output.flush()
}
```

`generate_sequences` prepares stdout, the visited-key set, and the mutable move
sequence used during recursion.

```rust
fn explore(
    position: Position,
    move_sequence: &mut String,
    depth: i32,
    visited_positions: &mut HashSet<u64>,
    output: &mut impl Write,
) -> io::Result<()> {
    if !visited_positions.insert(position.key3()) {
        return Ok(());
    }

    let move_count = position.move_count() as i32;
    if move_count <= depth {
        writeln!(output, "{move_sequence}")?;
    }
    if move_count >= depth {
        return Ok(());
    }
```

`explore` deduplicates by `key3`, prints positions at or above the requested
depth boundary, and stops recursion once the boundary is reached.

```rust
    for column in 0..WIDTH {
        if !position.can_play(column) || position.is_winning_move(column) {
            continue;
        }

        let mut next_position = position;
        next_position
            .play_col(column)
            .expect("column was just checked");
        move_sequence.push(char::from(b'1' + column as u8));
        explore(
            next_position,
            move_sequence,
            depth,
            visited_positions,
            output,
        )?;
        move_sequence.pop();
    }

    Ok(())
}
```

Only playable non-winning moves are expanded. The sequence string is updated
before recursion and restored after recursion.

### Opening-Book Generation

```rust
fn generate_opening_book() -> io::Result<()> {
    let capacity = next_prime(1_u64 << BOOK_LOG_SIZE) as usize;
    let mut keys = vec![0_u16; capacity];
    let mut values = vec![0_u8; capacity];
    let stdin = io::stdin();
```

Book mode allocates parallel key and value arrays sized by the prime table
capacity.

```rust
    for (line_index, line) in stdin.lock().lines().enumerate() {
        let line = line?;
        if line.is_empty() {
            break;
        }

        if let Err(message) = store_book_line(&line, &mut keys, &mut values) {
            eprintln!("Invalid line (ignored): {line}");
            eprintln!("{message}");
            continue;
        }

        let line_count = line_index + 1;
        if line_count.is_multiple_of(1_000_000) {
            eprintln!("{line_count}");
        }
    }

    write_book_file(OUTPUT_BOOK_FILE, &keys, &values)
}
```

Input stops at an empty line. Invalid lines are reported and ignored. Every
million input lines, the tool prints progress to stderr. At the end, it writes
`7x6.book`.

### Storing One Book Line

```rust
fn store_book_line(line: &str, keys: &mut [u16], values: &mut [u8]) -> Result<(), String> {
    let mut fields = line.split(' ');
    let sequence = fields
        .next()
        .ok_or_else(|| "missing position sequence".to_string())?;
    let score_text = fields.next().ok_or_else(|| "missing score".to_string())?;
    if fields.next().is_some() {
        return Err("too many fields".to_string());
    }

    let score = score_text
        .parse::<i32>()
        .map_err(|_| format!("invalid score: {score_text}"))?;
    if !(MIN_SCORE..=MAX_SCORE).contains(&score) {
        return Err(format!("score out of range: {score}"));
    }

    let position = Position::from_sequence(sequence).map_err(|error| error.to_string())?;
    let key = position.key3();
    let index = key as usize % keys.len();
    keys[index] = key as u16;
    values[index] = (score - MIN_SCORE + 1) as u8;

    Ok(())
}
```

A valid input line has exactly two fields: a move sequence and a score. The
score is encoded with the opening-book offset and stored at `key3 % capacity`.

### Writing A Book File

```rust
fn write_book_file(path: &str, keys: &[u16], values: &[u8]) -> io::Result<()> {
    let file = File::create(path)?;
    let mut output = BufWriter::new(file);

    output.write_all(&[
        WIDTH as u8,
        HEIGHT as u8,
        BOOK_DEPTH,
        BOOK_PARTIAL_KEY_BYTES,
        BOOK_VALUE_BYTES,
        BOOK_LOG_SIZE,
    ])?;
    for key in keys {
        output.write_all(&key.to_le_bytes())?;
    }
    output.write_all(values)?;
    output.flush()
}
```

The file layout is the six-byte opening-book header, all little-endian `u16`
partial keys, and all one-byte values.

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::{explore, store_book_line};
    use exact_connect4::position::{MIN_SCORE, Position};
    use std::collections::HashSet;

    #[test]
    fn depth_zero_outputs_empty_sequence() {
        assert_eq!(sequences_for_depth(0), "\n");
    }

    #[test]
    fn depth_one_outputs_unique_first_moves() {
        assert_eq!(sequences_for_depth(1), "\n1\n2\n3\n4\n");
    }

    #[test]
    fn depth_two_outputs_expected_prefix() {
        assert_eq!(
            sequences_for_depth(2),
            "\n1\n11\n12\n13\n14\n15\n16\n17\n2\n21\n22\n23\n24\n25\n26\n27\n3\n31\n32\n33\n34\n35\n36\n37\n4\n41\n42\n43\n44\n"
        );
    }

    #[test]
    fn store_book_line_encodes_score_offset() {
        let mut keys = vec![0_u16; 257];
        let mut values = vec![0_u8; 257];

        store_book_line("32164625 11", &mut keys, &mut values).unwrap();

        let key = Position::from_sequence("32164625").unwrap().key3();
        let index = key as usize % keys.len();
        assert_eq!(keys[index], key as u16);
        assert_eq!(values[index], (11 - MIN_SCORE + 1) as u8);
    }

    fn sequences_for_depth(depth: i32) -> String {
        let mut output = Vec::new();
        let mut visited_positions = HashSet::new();
        let mut move_sequence = String::new();

        explore(
            Position::new(),
            &mut move_sequence,
            depth,
            &mut visited_positions,
            &mut output,
        )
        .unwrap();

        String::from_utf8(output).unwrap()
    }
}
```

The tests cover depth 0, 1, and 2 output, plus score encoding for book lines.
