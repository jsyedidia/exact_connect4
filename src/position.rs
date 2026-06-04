// SPDX-FileCopyrightText: 2017-2019 Pascal Pons <contact@gamesolver.org>
// SPDX-FileCopyrightText: 2026 Jonathan Yedidia
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::error::Error;
use std::fmt;

/// Bitboard storage for the standard 7x6 Connect4 board.
pub type Bitboard = u64;

/// Number of columns in the board.
pub const WIDTH: usize = 7;

/// Number of playable rows in each column.
pub const HEIGHT: usize = 6;

/// Number of playable cells in the board.
pub const BOARD_SIZE: usize = WIDTH * HEIGHT;

/// Number of bits allocated to each column, including its sentinel bit.
pub const COLUMN_STRIDE: usize = HEIGHT + 1;

/// Minimum exact score on a 7x6 board.
pub const MIN_SCORE: i32 = -(BOARD_SIZE as i32) / 2 + 3;

/// Maximum exact score on a 7x6 board.
pub const MAX_SCORE: i32 = (BOARD_SIZE as i32 + 1) / 2 - 3;

const HORIZONTAL_SHIFT: usize = COLUMN_STRIDE;
const DIAGONAL_UP_SHIFT: usize = HEIGHT;
const DIAGONAL_DOWN_SHIFT: usize = HEIGHT + 2;

/// Bitboard containing the bottom playable cell of every column.
pub const BOTTOM_MASK: Bitboard = bottom_mask();

/// Bitboard containing every playable cell and no sentinel bits.
pub const BOARD_MASK: Bitboard = BOTTOM_MASK * ((1_u64 << HEIGHT) - 1);

/// A Connect4 position represented by two bitboards.
///
/// `mask` stores every occupied cell. `current_position` stores the stones
/// belonging to the side to move. This is Pascal Pons' representation from the
/// final C++ solver.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Position {
    current_position: Bitboard,
    mask: Bitboard,
    moves: usize,
}

/// Error returned when playing a column directly fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayColumnError {
    /// The column was outside the 0-based range `0..WIDTH`.
    InvalidColumn { column: usize },
    /// The column is already full.
    FullColumn { column: usize },
}

/// Error returned while parsing and playing a move sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaySequenceError {
    /// Zero-based index of the move that failed.
    pub move_index: usize,
    /// Specific reason the move failed.
    pub kind: PlaySequenceErrorKind,
}

/// Specific reason a sequence move failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaySequenceErrorKind {
    /// The move character was not an ASCII digit.
    InvalidCharacter { character: char },
    /// The digit was outside the 1-based range `1..=WIDTH`.
    InvalidColumn { character: char },
    /// The named column is already full.
    FullColumn { column: usize },
    /// The move would end the game, so it is not a valid non-terminal solver
    /// position.
    WinningMove { column: usize },
}

impl Position {
    /// Creates an empty board.
    ///
    /// # Examples
    ///
    /// ```
    /// let position = exact_connect4::Position::new();
    ///
    /// assert_eq!(position.move_count(), 0);
    /// assert_eq!(position.possible().count_ones(), exact_connect4::position::WIDTH as u32);
    /// ```
    #[must_use]
    pub const fn new() -> Self {
        Self {
            current_position: 0,
            mask: 0,
            moves: 0,
        }
    }

    /// Creates a position by playing a 1-based column sequence.
    ///
    /// # Examples
    ///
    /// ```
    /// let position = exact_connect4::Position::from_sequence("4453")?;
    ///
    /// assert_eq!(position.move_count(), 4);
    /// assert!(position.can_play(3));
    /// # Ok::<(), exact_connect4::PlaySequenceError>(())
    /// ```
    pub fn from_sequence(sequence: &str) -> Result<Self, PlaySequenceError> {
        let mut position = Self::new();
        position.play_sequence(sequence)?;
        Ok(position)
    }

    /// Plays a 1-based column sequence into this position.
    ///
    /// Moves before the failing move remain applied if this returns an error.
    /// Like the C++ reference parser, this rejects a move that would already
    /// win the game.
    pub fn play_sequence(&mut self, sequence: &str) -> Result<(), PlaySequenceError> {
        for (move_index, character) in sequence.chars().enumerate() {
            let column = parse_column_character(character)
                .map_err(|kind| PlaySequenceError { move_index, kind })?;

            if !self.can_play(column) {
                return Err(PlaySequenceError {
                    move_index,
                    kind: PlaySequenceErrorKind::FullColumn { column },
                });
            }
            if self.is_winning_move(column) {
                return Err(PlaySequenceError {
                    move_index,
                    kind: PlaySequenceErrorKind::WinningMove { column },
                });
            }
            self.play_col(column).expect("column was just validated");
        }
        Ok(())
    }

    /// Plays a move represented as a single-bit bitboard.
    ///
    /// The caller must provide a legal playable move.
    pub fn play_bitboard(&mut self, move_bit: Bitboard) {
        debug_assert_eq!(move_bit.count_ones(), 1);
        debug_assert_ne!(move_bit & self.possible(), 0);

        self.current_position ^= self.mask;
        self.mask |= move_bit;
        self.moves += 1;
    }

    /// Plays a zero-based column.
    ///
    /// This method allows winning moves. Sequence parsing rejects winning moves
    /// because parsed sequences represent non-terminal solver positions.
    ///
    /// # Examples
    ///
    /// ```
    /// let mut position = exact_connect4::Position::new();
    ///
    /// position.play_col(3)?;
    ///
    /// assert_eq!(position.move_count(), 1);
    /// # Ok::<(), exact_connect4::PlayColumnError>(())
    /// ```
    pub fn play_col(&mut self, column: usize) -> Result<(), PlayColumnError> {
        if column >= WIDTH {
            return Err(PlayColumnError::InvalidColumn { column });
        }
        if !self.can_play(column) {
            return Err(PlayColumnError::FullColumn { column });
        }

        self.play_bitboard((self.mask + bottom_mask_col(column)) & column_mask(column));
        Ok(())
    }

    /// Returns true if the side to move has an immediate winning move.
    #[must_use]
    pub fn can_win_next(&self) -> bool {
        (self.winning_position() & self.possible()) != 0
    }

    /// Returns the number of played moves.
    #[must_use]
    pub const fn move_count(&self) -> usize {
        self.moves
    }

    /// Returns the canonical bitboard key used by the transposition table.
    #[must_use]
    pub const fn key(&self) -> Bitboard {
        self.current_position + self.mask
    }

    /// Builds a symmetric base-3 key so mirrored positions share one key.
    #[must_use]
    pub fn key3(&self) -> u64 {
        let mut key_forward = 0;
        for column in 0..WIDTH {
            self.partial_key3(&mut key_forward, column);
        }

        let mut key_reverse = 0;
        for column in (0..WIDTH).rev() {
            self.partial_key3(&mut key_reverse, column);
        }

        if key_forward < key_reverse {
            key_forward / 3
        } else {
            key_reverse / 3
        }
    }

    /// Returns all legal moves.
    #[must_use]
    pub const fn possible(&self) -> Bitboard {
        (self.mask + BOTTOM_MASK) & BOARD_MASK
    }

    /// Returns legal moves that do not allow an immediate reply win.
    ///
    /// The side to move must not already have an immediate winning move.
    #[must_use]
    pub fn possible_non_losing_moves(&self) -> Bitboard {
        debug_assert!(!self.can_win_next());

        let mut playable_moves = self.possible();
        let opponent_wins = self.opponent_winning_position();
        let forced_moves = playable_moves & opponent_wins;

        if forced_moves != 0 {
            if (forced_moves & (forced_moves - 1)) != 0 {
                return 0;
            }
            playable_moves = forced_moves;
        }

        playable_moves & !(opponent_wins >> 1)
    }

    /// Scores a move by counting how many winning cells it creates.
    #[must_use]
    pub fn move_score(&self, move_bit: Bitboard) -> i32 {
        compute_winning_position(self.current_position | move_bit, self.mask).count_ones() as i32
    }

    /// Returns true if a zero-based column is playable.
    #[must_use]
    pub fn can_play(&self, column: usize) -> bool {
        column < WIDTH && (self.mask & top_mask_col(column)) == 0
    }

    /// Returns true if playing a zero-based column would win immediately.
    #[must_use]
    pub fn is_winning_move(&self, column: usize) -> bool {
        column < WIDTH
            && self.can_play(column)
            && (self.winning_position() & self.possible() & column_mask(column)) != 0
    }

    fn partial_key3(&self, key: &mut u64, column: usize) {
        let mut position = bottom_mask_col(column);
        while (position & self.mask) != 0 {
            *key *= 3;
            *key += if (position & self.current_position) != 0 {
                1
            } else {
                2
            };
            position <<= 1;
        }
        *key *= 3;
    }

    fn winning_position(&self) -> Bitboard {
        compute_winning_position(self.current_position, self.mask)
    }

    fn opponent_winning_position(&self) -> Bitboard {
        compute_winning_position(self.current_position ^ self.mask, self.mask)
    }
}

impl fmt::Display for PlayColumnError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::InvalidColumn { column } => {
                write!(formatter, "invalid column {}", column + 1)
            }
            Self::FullColumn { column } => {
                write!(formatter, "column {} is full", column + 1)
            }
        }
    }
}

impl Error for PlayColumnError {}

impl fmt::Display for PlaySequenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid move {}: {}",
            self.move_index + 1,
            self.kind
        )
    }
}

impl Error for PlaySequenceError {}

impl fmt::Display for PlaySequenceErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Self::InvalidCharacter { character } => {
                write!(formatter, "invalid character {character:?}")
            }
            Self::InvalidColumn { character } => {
                write!(formatter, "invalid column digit {character:?}")
            }
            Self::FullColumn { column } => {
                write!(formatter, "column {} is full", column + 1)
            }
            Self::WinningMove { column } => {
                write!(formatter, "column {} would end the game", column + 1)
            }
        }
    }
}

impl Error for PlaySequenceErrorKind {}

/// Returns a bitboard containing all playable cells in a zero-based column.
#[must_use]
pub const fn column_mask(column: usize) -> Bitboard {
    assert!(column < WIDTH);
    ((1_u64 << HEIGHT) - 1) << (column * COLUMN_STRIDE)
}

/// Returns the top playable cell in a zero-based column.
#[must_use]
pub const fn top_mask_col(column: usize) -> Bitboard {
    assert!(column < WIDTH);
    1_u64 << ((HEIGHT - 1) + column * COLUMN_STRIDE)
}

/// Returns the bottom playable cell in a zero-based column.
#[must_use]
pub const fn bottom_mask_col(column: usize) -> Bitboard {
    assert!(column < WIDTH);
    1_u64 << (column * COLUMN_STRIDE)
}

const fn bottom_mask() -> Bitboard {
    let mut mask = 0;
    let mut column = 0;
    while column < WIDTH {
        mask |= 1_u64 << (column * COLUMN_STRIDE);
        column += 1;
    }
    mask
}

fn parse_column_character(character: char) -> Result<usize, PlaySequenceErrorKind> {
    if !character.is_ascii_digit() {
        return Err(PlaySequenceErrorKind::InvalidCharacter { character });
    }
    let Some(digit) = character.to_digit(10) else {
        return Err(PlaySequenceErrorKind::InvalidCharacter { character });
    };
    if !(1..=WIDTH as u32).contains(&digit) {
        return Err(PlaySequenceErrorKind::InvalidColumn { character });
    }
    Ok(digit as usize - 1)
}

fn compute_winning_position(position: Bitboard, occupied_mask: Bitboard) -> Bitboard {
    let mut result = (position << 1) & (position << 2) & (position << 3);

    let mut pairs = (position << HORIZONTAL_SHIFT) & (position << (2 * HORIZONTAL_SHIFT));
    result |= pairs & (position << (3 * HORIZONTAL_SHIFT));
    result |= pairs & (position >> HORIZONTAL_SHIFT);
    pairs = (position >> HORIZONTAL_SHIFT) & (position >> (2 * HORIZONTAL_SHIFT));
    result |= pairs & (position << HORIZONTAL_SHIFT);
    result |= pairs & (position >> (3 * HORIZONTAL_SHIFT));

    pairs = (position << DIAGONAL_UP_SHIFT) & (position << (2 * DIAGONAL_UP_SHIFT));
    result |= pairs & (position << (3 * DIAGONAL_UP_SHIFT));
    result |= pairs & (position >> DIAGONAL_UP_SHIFT);
    pairs = (position >> DIAGONAL_UP_SHIFT) & (position >> (2 * DIAGONAL_UP_SHIFT));
    result |= pairs & (position << DIAGONAL_UP_SHIFT);
    result |= pairs & (position >> (3 * DIAGONAL_UP_SHIFT));

    pairs = (position << DIAGONAL_DOWN_SHIFT) & (position << (2 * DIAGONAL_DOWN_SHIFT));
    result |= pairs & (position << (3 * DIAGONAL_DOWN_SHIFT));
    result |= pairs & (position >> DIAGONAL_DOWN_SHIFT);
    pairs = (position >> DIAGONAL_DOWN_SHIFT) & (position >> (2 * DIAGONAL_DOWN_SHIFT));
    result |= pairs & (position << DIAGONAL_DOWN_SHIFT);
    result |= pairs & (position >> (3 * DIAGONAL_DOWN_SHIFT));

    result & (BOARD_MASK ^ occupied_mask)
}

#[cfg(test)]
mod tests {
    use super::{
        BOARD_MASK, BOTTOM_MASK, Bitboard, HEIGHT, PlayColumnError, PlaySequenceError,
        PlaySequenceErrorKind, Position, WIDTH, bottom_mask_col, column_mask, top_mask_col,
    };

    #[test]
    fn masks_match_the_documented_bit_layout() {
        let expected_bottom = (0..WIDTH)
            .map(|column| 1_u64 << (column * (HEIGHT + 1)))
            .fold(0, |mask, bit| mask | bit);
        assert_eq!(BOTTOM_MASK, expected_bottom);

        for column in 0..WIDTH {
            assert_eq!(bottom_mask_col(column), 1_u64 << (column * (HEIGHT + 1)));
            assert_eq!(
                top_mask_col(column),
                1_u64 << (HEIGHT - 1 + column * (HEIGHT + 1))
            );

            let mut expected_column = 0;
            for row in 0..HEIGHT {
                expected_column |= 1_u64 << (row + column * (HEIGHT + 1));
            }
            assert_eq!(column_mask(column), expected_column);
            assert_eq!(BOARD_MASK & expected_column, expected_column);
        }
    }

    #[test]
    fn plays_short_sequence_and_tracks_state() {
        let position = Position::from_sequence("4453").unwrap();
        let expected_mask = bit(3, 0) | bit(3, 1) | bit(4, 0) | bit(2, 0);
        let expected_current = bit(3, 0) | bit(4, 0);

        assert_eq!(position.move_count(), 4);
        assert_eq!(position.mask, expected_mask);
        assert_eq!(position.current_position, expected_current);
        assert!(position.can_play(3));
    }

    #[test]
    fn rejects_bad_sequence_characters_and_columns() {
        assert_sequence_error(
            Position::from_sequence("12a4"),
            2,
            PlaySequenceErrorKind::InvalidCharacter { character: 'a' },
        );
        assert_sequence_error(
            Position::from_sequence("1238"),
            3,
            PlaySequenceErrorKind::InvalidColumn { character: '8' },
        );
    }

    #[test]
    fn rejects_full_columns_and_winning_moves_in_sequences() {
        assert_sequence_error(
            Position::from_sequence("1111111"),
            6,
            PlaySequenceErrorKind::FullColumn { column: 0 },
        );
        assert_sequence_error(
            Position::from_sequence("1212121"),
            6,
            PlaySequenceErrorKind::WinningMove { column: 0 },
        );
    }

    #[test]
    fn play_col_checks_bounds_and_full_columns() {
        let mut position = Position::new();
        assert_eq!(
            position.play_col(WIDTH),
            Err(PlayColumnError::InvalidColumn { column: WIDTH })
        );

        for _ in 0..HEIGHT {
            position.play_col(0).unwrap();
        }
        assert!(!position.can_play(0));
        assert_eq!(
            position.play_col(0),
            Err(PlayColumnError::FullColumn { column: 0 })
        );
    }

    #[test]
    fn accepts_a_full_board_draw_sequence() {
        let draw = "177322644317353514472267227353611516544566";
        let position = Position::from_sequence(draw).unwrap();
        assert_eq!(position.move_count(), WIDTH * HEIGHT);
        assert_eq!(position.possible(), 0);
    }

    #[test]
    fn detects_immediate_wins() {
        let position = Position::from_sequence("121212").unwrap();
        assert!(position.can_win_next());
        assert!(position.is_winning_move(0));
        assert!(!position.is_winning_move(1));
    }

    #[test]
    fn filters_non_losing_moves_to_forced_block() {
        let position = Position::from_sequence("12121").unwrap();
        assert!(!position.can_win_next());
        assert_eq!(position.possible_non_losing_moves(), bit(0, 3));
    }

    #[test]
    fn symmetric_positions_have_the_same_key3() {
        let position = Position::from_sequence("4453").unwrap();
        let mirrored = Position::from_sequence("4435").unwrap();
        assert_eq!(position.key3(), mirrored.key3());
    }

    #[test]
    fn selected_benchmark_positions_parse() {
        for sequence in include_str!("../tests/fixtures/positions/Seq_L1_R1")
            .lines()
            .take(32)
        {
            Position::from_sequence(sequence).unwrap();
        }
    }

    fn bit(column: usize, row: usize) -> Bitboard {
        1_u64 << (row + column * (HEIGHT + 1))
    }

    fn assert_sequence_error(
        result: Result<Position, PlaySequenceError>,
        move_index: usize,
        kind: PlaySequenceErrorKind,
    ) {
        let error = result.expect_err("sequence should fail");
        assert_eq!(error.move_index, move_index);
        assert_eq!(error.kind, kind);
    }
}
