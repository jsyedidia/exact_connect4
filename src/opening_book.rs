// SPDX-FileCopyrightText: 2017-2019 Pascal Pons <contact@gamesolver.org>
// SPDX-FileCopyrightText: 2026 Jonathan Yedidia
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::error::Error;
use std::fmt;
use std::fs;
use std::path::Path;

use crate::position::{BOARD_SIZE, HEIGHT, Position, WIDTH};
use crate::transposition_table::next_prime;

const HEADER_SIZE: usize = 6;
const MIN_LOG_SIZE: u8 = 21;
const MAX_LOG_SIZE: u8 = 27;
const VALUE_BYTES: u8 = 1;

/// Opening-book lookup table loaded from a compatible book file.
#[derive(Debug, Clone, Default)]
pub struct OpeningBook {
    depth: Option<usize>,
    table: Option<BookTable>,
}

impl OpeningBook {
    /// Creates an unloaded opening book.
    ///
    /// # Examples
    ///
    /// ```
    /// let book = exact_connect4::OpeningBook::new();
    ///
    /// assert_eq!(book.depth(), None);
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self {
            depth: None,
            table: None,
        }
    }

    /// Loads the tracked default opening book embedded in this crate.
    ///
    /// # Examples
    ///
    /// ```
    /// let book = exact_connect4::OpeningBook::from_default_book()?;
    ///
    /// assert_eq!(book.depth(), Some(14));
    /// assert!(book.capacity().is_some());
    /// # Ok::<(), exact_connect4::OpeningBookError>(())
    /// ```
    pub fn from_default_book() -> Result<Self, OpeningBookError> {
        Self::from_bytes(crate::default_opening_book_bytes())
    }

    /// Loads an opening book from a file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, OpeningBookError> {
        let bytes = fs::read(path).map_err(OpeningBookError::Io)?;
        Self::from_bytes(&bytes)
    }

    /// Loads an opening book from its binary bytes.
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

    /// Returns the raw stored book value for a position, or zero on a miss.
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

    /// Returns the maximum move depth stored in the book.
    #[must_use]
    pub const fn depth(&self) -> Option<usize> {
        self.depth
    }

    /// Returns the number of slots in the loaded book table.
    #[must_use]
    pub fn capacity(&self) -> Option<usize> {
        self.table.as_ref().map(BookTable::capacity)
    }

    /// Returns the partial-key byte width of the loaded book table.
    #[must_use]
    pub fn partial_key_bytes(&self) -> Option<usize> {
        self.table.as_ref().map(BookTable::partial_key_bytes)
    }
}

#[derive(Debug, Clone)]
enum BookTable {
    U8 { keys: Vec<u8>, values: Vec<u8> },
    U16 { keys: Vec<u16>, values: Vec<u8> },
    U32 { keys: Vec<u32>, values: Vec<u8> },
}

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

/// Error returned when opening-book loading fails.
#[derive(Debug)]
pub enum OpeningBookError {
    /// File IO failed.
    Io(std::io::Error),
    /// The file ended before the six-byte header was complete.
    TruncatedHeader { found: usize, expected: usize },
    /// The book was built for a different board width.
    InvalidWidth { found: u8, expected: u8 },
    /// The book was built for a different board height.
    InvalidHeight { found: u8, expected: u8 },
    /// The stored depth is larger than the board.
    InvalidDepth { found: u8, max: u8 },
    /// The partial-key byte width is not supported by this loader.
    UnsupportedKeySize { found: u8 },
    /// Stored values must be one byte each.
    InvalidValueSize { found: u8, expected: u8 },
    /// The log table size is outside the supported range.
    UnsupportedLogSize { found: u8, min: u8, max: u8 },
    /// The data section ended before all keys and values were present.
    TruncatedData { found: usize, expected: usize },
}

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

impl Error for OpeningBookError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

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
