# src/opening_book.rs

## Role In The System

`src/opening_book.rs` loads and queries compatible binary opening-book files.
The default solver uses this module to load the embedded `data/books/7x6.book`
and avoid expensive early-game search when a position is stored in the book.

The loader validates the file header, decodes key storage without unsafe code,
and returns explicit errors for malformed data.

Related concept notes:

- [notes/concepts/opening_books.md](../concepts/opening_books.md)
- [notes/concepts/bitboards.md](../concepts/bitboards.md)
- [notes/concepts/transposition_tables.md](../concepts/transposition_tables.md)
- [notes/concepts/solver_scores.md](../concepts/solver_scores.md)

## Code Walkthrough

### Imports

```rust
use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;

use crate::position::{BOARD_SIZE, HEIGHT, Position, WIDTH};
use crate::transposition_table::next_prime;
```

The module uses standard error and filesystem APIs, board constants for header
validation, `Position` for lookup, and `next_prime` for table sizing.

### Format Constants

```rust
const HEADER_SIZE: usize = 6;
const MIN_LOG_SIZE: u8 = 21;
const MAX_LOG_SIZE: u8 = 27;
const VALUE_BYTES: u8 = 1;
```

These constants describe the supported book format. The tracked book uses a log
size in this range and stores one-byte values.

### OpeningBook

```rust
#[derive(Debug, Clone, Default)]
pub struct OpeningBook {
    depth: Option<usize>,
    table: Option<BookTable>,
}
```

`depth` is `None` when no book is loaded. `table` stores the decoded key and
value arrays.

### Constructors

```rust
impl OpeningBook {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            depth: None,
            table: None,
        }
    }

    pub fn from_default_book() -> Result<Self, OpeningBookError> {
        Self::from_bytes(crate::default_opening_book_bytes())
    }

    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, OpeningBookError> {
        let bytes = fs::read(path).map_err(OpeningBookError::Io)?;
        Self::from_bytes(&bytes)
    }
```

`new` creates an unloaded book. `from_default_book` parses the embedded default
book bytes. `from_file` reads a file and then delegates to the byte parser.

### Header Validation

```rust
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, OpeningBookError> {
        if bytes.len() < HEADER_SIZE {
            return Err(OpeningBookError::TruncatedHeader {
                found: bytes.len(),
                expected: HEADER_SIZE,
            });
        }

        let width = bytes[0];
        if width != WIDTH as u8 {
            return Err(OpeningBookError::InvalidWidth {
                found: width,
                expected: WIDTH as u8,
            });
        }

        let height = bytes[1];
        if height != HEIGHT as u8 {
            return Err(OpeningBookError::InvalidHeight {
                found: height,
                expected: HEIGHT as u8,
            });
        }

        let depth = bytes[2];
        if depth as usize > BOARD_SIZE {
            return Err(OpeningBookError::InvalidDepth {
                found: depth,
                max: BOARD_SIZE as u8,
            });
        }

        let partial_key_bytes = bytes[3];
        if !matches!(partial_key_bytes, 1 | 2 | 4) {
            return Err(OpeningBookError::UnsupportedKeySize {
                found: partial_key_bytes,
            });
        }

        let value_bytes = bytes[4];
        if value_bytes != VALUE_BYTES {
            return Err(OpeningBookError::InvalidValueSize {
                found: value_bytes,
                expected: VALUE_BYTES,
            });
        }

        let log_size = bytes[5];
        if !(MIN_LOG_SIZE..=MAX_LOG_SIZE).contains(&log_size) {
            return Err(OpeningBookError::UnsupportedLogSize {
                found: log_size,
                min: MIN_LOG_SIZE,
                max: MAX_LOG_SIZE,
            });
        }
```

The parser first validates the six-byte header. It rejects wrong board
dimensions, impossible depths, unsupported partial-key sizes, value sizes other
than one byte, and unsupported table log sizes.

### Data Slicing

```rust
        let capacity = next_prime(1_u64 << log_size) as usize;
        let key_bytes_len = capacity * partial_key_bytes as usize;
        let values_offset = HEADER_SIZE + key_bytes_len;
        let expected_len = values_offset + capacity * value_bytes as usize;
        if bytes.len() < expected_len {
            return Err(OpeningBookError::TruncatedData {
                found: bytes.len(),
                expected: expected_len,
            });
        }

        let key_bytes = &bytes[HEADER_SIZE..values_offset];
        let value_bytes = &bytes[values_offset..expected_len];
        let table = BookTable::from_parts(partial_key_bytes, capacity, key_bytes, value_bytes);

        Ok(Self {
            depth: Some(depth as usize),
            table: Some(table),
        })
    }
```

Once the header is valid, the parser computes the table capacity and slices the
key and value regions. The file may contain more bytes than required; the loader
uses the format-defined prefix.

### Lookup And Introspection

```rust
    #[must_use]
    pub fn get(&self, position: &Position) -> u8 {
        let Some(depth) = self.depth else {
            return 0;
        };
        if position.move_count() > depth {
            return 0;
        }
        let Some(table) = &self.table else {
            return 0;
        };
        table.get(position.key3())
    }

    #[must_use]
    pub const fn depth(&self) -> Option<usize> {
        self.depth
    }

    #[must_use]
    pub fn capacity(&self) -> Option<usize> {
        self.table.as_ref().map(BookTable::capacity)
    }

    #[must_use]
    pub fn partial_key_bytes(&self) -> Option<usize> {
        self.table.as_ref().map(BookTable::partial_key_bytes)
    }
}
```

`get` returns zero for unloaded books, positions deeper than the book, missing
table entries, and stored zero values. The other methods expose header-derived
metadata for tests and diagnostics.

### BookTable

```rust
#[derive(Debug, Clone)]
enum BookTable {
    U8 { keys: Vec<u8>, values: Vec<u8> },
    U16 { keys: Vec<u16>, values: Vec<u8> },
    U32 { keys: Vec<u32>, values: Vec<u8> },
}
```

The table variants represent the supported partial-key widths. Values are
always one byte.

### Decoding Key Storage

```rust
impl BookTable {
    fn from_parts(partial_key_bytes: u8, capacity: usize, key_bytes: &[u8], values: &[u8]) -> Self {
        debug_assert_eq!(values.len(), capacity);
        match partial_key_bytes {
            1 => {
                debug_assert_eq!(key_bytes.len(), capacity);
                Self::U8 {
                    keys: key_bytes.to_vec(),
                    values: values.to_vec(),
                }
            }
            2 => Self::U16 {
                keys: key_bytes
                    .chunks_exact(2)
                    .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]))
                    .collect(),
                values: values.to_vec(),
            },
            4 => Self::U32 {
                keys: key_bytes
                    .chunks_exact(4)
                    .map(|bytes| u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
                    .collect(),
                values: values.to_vec(),
            },
            _ => unreachable!("partial key size was already validated"),
        }
    }
```

One-byte keys can be copied directly. Wider keys are decoded as little-endian
integers.

### Table Lookup

```rust
    fn get(&self, key: u64) -> u8 {
        match self {
            Self::U8 { keys, values } => {
                let index = key as usize % keys.len();
                if keys[index] == key as u8 {
                    values[index]
                } else {
                    0
                }
            }
            Self::U16 { keys, values } => {
                let index = key as usize % keys.len();
                if keys[index] == key as u16 {
                    values[index]
                } else {
                    0
                }
            }
            Self::U32 { keys, values } => {
                let index = key as usize % keys.len();
                if keys[index] == key as u32 {
                    values[index]
                } else {
                    0
                }
            }
        }
    }
```

The lookup computes one slot, compares the stored partial key with the requested
key truncated to the same width, and returns the stored value on a match.

### Table Metadata

```rust
    fn capacity(&self) -> usize {
        match self {
            Self::U8 { keys, .. } => keys.len(),
            Self::U16 { keys, .. } => keys.len(),
            Self::U32 { keys, .. } => keys.len(),
        }
    }

    const fn partial_key_bytes(&self) -> usize {
        match self {
            Self::U8 { .. } => 1,
            Self::U16 { .. } => 2,
            Self::U32 { .. } => 4,
        }
    }
}
```

These helpers report the decoded table capacity and key width.

### Error Type

```rust
#[derive(Debug)]
pub enum OpeningBookError {
    Io(std::io::Error),
    TruncatedHeader { found: usize, expected: usize },
    InvalidWidth { found: u8, expected: u8 },
    InvalidHeight { found: u8, expected: u8 },
    InvalidDepth { found: u8, max: u8 },
    UnsupportedKeySize { found: u8 },
    InvalidValueSize { found: u8, expected: u8 },
    UnsupportedLogSize { found: u8, min: u8, max: u8 },
    TruncatedData { found: usize, expected: usize },
}
```

The error variants distinguish filesystem failures from each supported
validation failure.

```rust
impl fmt::Display for OpeningBookError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "{error}"),
            Self::TruncatedHeader { found, expected } => {
                write!(
                    formatter,
                    "truncated opening-book header: found {found} bytes, expected {expected}"
                )
            }
            Self::InvalidWidth { found, expected } => {
                write!(
                    formatter,
                    "invalid opening-book width: found {found}, expected {expected}"
                )
            }
            Self::InvalidHeight { found, expected } => {
                write!(
                    formatter,
                    "invalid opening-book height: found {found}, expected {expected}"
                )
            }
            Self::InvalidDepth { found, max } => {
                write!(
                    formatter,
                    "invalid opening-book depth: found {found}, max {max}"
                )
            }
            Self::UnsupportedKeySize { found } => {
                write!(
                    formatter,
                    "unsupported opening-book partial-key size: {found} bytes"
                )
            }
            Self::InvalidValueSize { found, expected } => {
                write!(
                    formatter,
                    "invalid opening-book value size: found {found}, expected {expected}"
                )
            }
            Self::UnsupportedLogSize { found, min, max } => {
                write!(
                    formatter,
                    "unsupported opening-book log size: found {found}, expected {min}..={max}"
                )
            }
            Self::TruncatedData { found, expected } => {
                write!(
                    formatter,
                    "truncated opening-book data: found {found} bytes, expected {expected}"
                )
            }
        }
    }
}
```

`Display` turns each error into a human-readable message.

```rust
impl Error for OpeningBookError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}
```

The `Error` implementation exposes the underlying IO error when one exists.

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::{OpeningBook, OpeningBookError};
    use crate::position::{MIN_SCORE, Position};
    use crate::transposition_table::next_prime;

    #[test]
    fn default_book_loads_with_expected_header_values() {
        let book = OpeningBook::from_default_book().unwrap();

        assert_eq!(book.depth(), Some(14));
        assert_eq!(book.partial_key_bytes(), Some(1));
        assert_eq!(book.capacity(), Some(next_prime(1 << 24) as usize));
    }

    #[test]
    fn default_book_matches_known_fixture_score() {
        let book = OpeningBook::from_default_book().unwrap();

        let line = include_str!("../tests/fixtures/benchmarks/Test_L1_R1")
            .lines()
            .next()
            .unwrap();
        let (sequence, expected_score) = line.split_once(' ').unwrap();
        let position = Position::from_sequence(sequence).unwrap();
        let expected_score = expected_score.parse::<i32>().unwrap();
        let expected_value = (expected_score - MIN_SCORE + 1) as u8;

        assert_eq!(book.get(&position), expected_value, "{sequence}");
    }

    #[test]
    fn positions_deeper_than_book_depth_miss() {
        let book = OpeningBook::from_default_book().unwrap();
        let position = Position::from_sequence("2252576253462244111563365343671351441").unwrap();

        assert_eq!(book.get(&position), 0);
    }

    #[test]
    fn unloaded_book_misses() {
        let book = OpeningBook::new();
        let position = Position::new();

        assert_eq!(book.get(&position), 0);
    }

    #[test]
    fn rejects_truncated_header() {
        let error = OpeningBook::from_bytes(&[7, 6, 14]).unwrap_err();

        assert!(matches!(
            error,
            OpeningBookError::TruncatedHeader {
                found: 3,
                expected: 6
            }
        ));
    }

    #[test]
    fn rejects_bad_header_fields() {
        assert!(matches!(
            OpeningBook::from_bytes(&[8, 6, 14, 1, 1, 24]).unwrap_err(),
            OpeningBookError::InvalidWidth { found: 8, .. }
        ));
        assert!(matches!(
            OpeningBook::from_bytes(&[7, 7, 14, 1, 1, 24]).unwrap_err(),
            OpeningBookError::InvalidHeight { found: 7, .. }
        ));
        assert!(matches!(
            OpeningBook::from_bytes(&[7, 6, 43, 1, 1, 24]).unwrap_err(),
            OpeningBookError::InvalidDepth { found: 43, .. }
        ));
        assert!(matches!(
            OpeningBook::from_bytes(&[7, 6, 14, 8, 1, 24]).unwrap_err(),
            OpeningBookError::UnsupportedKeySize { found: 8 }
        ));
        assert!(matches!(
            OpeningBook::from_bytes(&[7, 6, 14, 1, 2, 24]).unwrap_err(),
            OpeningBookError::InvalidValueSize { found: 2, .. }
        ));
        assert!(matches!(
            OpeningBook::from_bytes(&[7, 6, 14, 1, 1, 20]).unwrap_err(),
            OpeningBookError::UnsupportedLogSize { found: 20, .. }
        ));
    }

    #[test]
    fn rejects_truncated_data() {
        let error = OpeningBook::from_bytes(&[7, 6, 14, 1, 1, 21]).unwrap_err();

        assert!(matches!(
            error,
            OpeningBookError::TruncatedData { found: 6, .. }
        ));
    }
}
```

The tests cover successful default loading, a known default-book hit, depth
misses, unloaded misses, malformed headers, and truncated data.
