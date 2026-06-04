# src/position.rs

## Role In The System

`src/position.rs` defines the board representation used by the rest of the
solver. It owns the board constants, bitboard masks, position state, move
parsing, move generation, immediate-win detection, non-losing move filtering,
move scoring, and position keys.

It deliberately does not perform game-tree search. The search code can stay
focused on alpha-beta logic because this file provides compact operations for
asking "what moves are legal?", "does this move win?", and "what is this
position's key?"

Related concept notes:

- [notes/concepts/connect4_rules.md](../concepts/connect4_rules.md)
- [notes/concepts/bitboards.md](../concepts/bitboards.md)
- [notes/concepts/solver_scores.md](../concepts/solver_scores.md)

## Main Types And State

The file uses `Bitboard = u64` for the 49-bit board encoding. `Position` is the
central type. It stores:

```rust
current_position: Bitboard,
mask: Bitboard,
moves: usize,
```

`mask` marks every occupied cell. `current_position` marks only the stones that
belong to the player whose turn it is. `moves` counts how many moves have been
played.

The error types are small value types:

- `PlayColumnError` describes direct column-play failures.
- `PlaySequenceError` adds the failing move index when parsing a sequence.
- `PlaySequenceErrorKind` describes the specific sequence failure.

## State At A Glance

The key state transition is in `play_bitboard`:

```rust
self.current_position ^= self.mask;
self.mask |= move_bit;
self.moves += 1;
```

The first line switches the stored player perspective. The second line marks
the newly occupied cell. The third line advances the move count.

## Code Walkthrough

### Imports

```rust
use std::error::Error;
use std::fmt;
```

The file implements standard error traits for its parse and play errors.
`std::fmt` supplies `Display` support, and `std::error::Error` marks the custom
errors as normal Rust errors.

### Board Constants

```rust
pub type Bitboard = u64;

pub const WIDTH: usize = 7;

pub const HEIGHT: usize = 6;

pub const BOARD_SIZE: usize = WIDTH * HEIGHT;

pub const COLUMN_STRIDE: usize = HEIGHT + 1;

pub const MIN_SCORE: i32 = -(BOARD_SIZE as i32) / 2 + 3;

pub const MAX_SCORE: i32 = (BOARD_SIZE as i32 + 1) / 2 - 3;
```

`WIDTH`, `HEIGHT`, and `BOARD_SIZE` define the standard Connect4 board.
`COLUMN_STRIDE` is `7`: six playable cells plus one sentinel bit per column.

`MIN_SCORE` and `MAX_SCORE` define the exact-score range for the standard
board. They live here because score limits depend only on board size.

### Direction Shifts

```rust
const HORIZONTAL_SHIFT: usize = COLUMN_STRIDE;
const DIAGONAL_UP_SHIFT: usize = HEIGHT;
const DIAGONAL_DOWN_SHIFT: usize = HEIGHT + 2;
```

These constants name the bit shifts used when finding potential winning cells.
Vertical adjacency uses shift `1`, so it does not need a named constant.

### Global Masks

```rust
pub const BOTTOM_MASK: Bitboard = bottom_mask();

pub const BOARD_MASK: Bitboard = BOTTOM_MASK * ((1_u64 << HEIGHT) - 1);
```

`BOTTOM_MASK` has one bit set at the bottom of each column. `BOARD_MASK` expands
those bottom bits upward through the six playable rows and excludes sentinel
bits.

### `Position`

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Position {
    current_position: Bitboard,
    mask: Bitboard,
    moves: usize,
}
```

`Position` is `Copy` so search can cheaply create child positions by value. The
fields are private; callers use methods such as `possible`, `can_play`, and
`key` rather than manipulating bitboards directly.

### `PlayColumnError`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayColumnError {
    InvalidColumn { column: usize },
    FullColumn { column: usize },
}
```

Direct column play can fail because the column is outside `0..WIDTH` or because
the column is already full.

### `PlaySequenceError`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaySequenceError {
    pub move_index: usize,
    pub kind: PlaySequenceErrorKind,
}
```

A sequence error records which move failed and why. The index is zero-based so
it can be used naturally with Rust string/iterator positions; display output
adds one for human-facing messages.

### `PlaySequenceErrorKind`

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaySequenceErrorKind {
    InvalidCharacter { character: char },
    InvalidColumn { character: char },
    FullColumn { column: usize },
    WinningMove { column: usize },
}
```

The sequence parser separates non-digits from digit characters outside the
legal `1..=7` range. It also distinguishes a full column from a move that would
finish the game.

### `impl Position`

```rust
impl Position {
```

The main implementation block contains constructors, state transitions, query
methods, and private helpers.

### `new`

```rust
    #[must_use]
    pub const fn new() -> Self {
        Self {
            current_position: 0,
            mask: 0,
            moves: 0,
        }
    }
```

`new` constructs the empty board. It is `const`, so callers can use it in
constant contexts if needed. `#[must_use]` warns when the new position is
created and then immediately discarded.

### `from_sequence`

```rust
    pub fn from_sequence(sequence: &str) -> Result<Self, PlaySequenceError> {
        let mut position = Self::new();
        position.play_sequence(sequence)?;
        Ok(position)
    }
```

`from_sequence` is the convenient parse constructor. It creates an empty
position, delegates the actual parsing to `play_sequence`, and returns the
finished position.

### `play_sequence`

```rust
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
```

The sequence parser walks one character at a time, converting the user-facing
`1` through `7` notation into a zero-based column. Parse errors are wrapped with
the current move index.

After parsing a column, the method rejects full columns and moves that would
already win the game. If those checks pass, `play_col` should succeed, so the
`expect` documents that this path has already validated the column.

### `play_bitboard`

```rust
    pub fn play_bitboard(&mut self, move_bit: Bitboard) {
        debug_assert_eq!(move_bit.count_ones(), 1);
        debug_assert_ne!(move_bit & self.possible(), 0);

        self.current_position ^= self.mask;
        self.mask |= move_bit;
        self.moves += 1;
    }
```

`play_bitboard` is the low-level move operation. The debug assertions state the
preconditions: the move is exactly one bit, and that bit is currently playable.

The state update first switches player perspective, then records the new
occupied bit, then increments the move count.

### `play_col`

```rust
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
```

`play_col` is a checked column wrapper around `play_bitboard`. The move bit is
computed by adding the column bottom bit to `mask`, which carries to the first
empty cell in that column, then masking away all other columns.

### `can_win_next`

```rust
    #[must_use]
    pub fn can_win_next(&self) -> bool {
        (self.winning_position() & self.possible()) != 0
    }
```

`can_win_next` asks whether any playable move is also one of the current
player's winning cells.

### `move_count`

```rust
    #[must_use]
    pub const fn move_count(&self) -> usize {
        self.moves
    }
```

`move_count` exposes the number of moves played so far.

### `key`

```rust
    #[must_use]
    pub const fn key(&self) -> Bitboard {
        self.current_position + self.mask
    }
```

`key` creates the compact key used by later transposition-table code.

### `key3`

```rust
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
```

`key3` builds a base-3 key from left to right and right to left. Returning the
smaller value makes mirrored positions share the same key. The final division
removes the extra trailing empty-cell marker added by `partial_key3`.

### `possible`

```rust
    #[must_use]
    pub const fn possible(&self) -> Bitboard {
        (self.mask + BOTTOM_MASK) & BOARD_MASK
    }
```

`possible` returns all legal move bits. Adding `BOTTOM_MASK` lifts each
column's bottom bit to the first empty cell; `BOARD_MASK` removes sentinel bits
and all non-board bits.

### `possible_non_losing_moves`

```rust
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
```

This method filters playable moves to those that do not allow an immediate
reply win. If the opponent has a playable winning cell, the current player must
block it. More than one forced block means no move can stop all immediate
threats, so the method returns `0`.

The final expression removes moves underneath opponent winning cells, because
those moves would give the opponent access to the winning cell on the next
turn.

### `move_score`

```rust
    #[must_use]
    pub fn move_score(&self, move_bit: Bitboard) -> i32 {
        compute_winning_position(self.current_position | move_bit, self.mask).count_ones() as i32
    }
```

`move_score` is a move-ordering heuristic. It imagines adding `move_bit` to the
current player's stones and counts how many future winning cells that creates.

### `can_play`

```rust
    #[must_use]
    pub fn can_play(&self, column: usize) -> bool {
        column < WIDTH && (self.mask & top_mask_col(column)) == 0
    }
```

A column is playable if it is in range and its top playable cell is not already
occupied.

### `is_winning_move`

```rust
    #[must_use]
    pub fn is_winning_move(&self, column: usize) -> bool {
        column < WIDTH
            && self.can_play(column)
            && (self.winning_position() & self.possible() & column_mask(column)) != 0
    }
```

`is_winning_move` checks that the column is valid, playable, and intersects a
winning cell for the current player.

### `partial_key3`

```rust
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
```

`partial_key3` walks one column from bottom upward while cells are occupied.
Each occupied cell appends a base-3 digit to `key`: `1` for the side to move
and `2` for the other side. The final multiplication appends the empty-cell
marker for the first open cell in the column.

### Winning-Position Helpers

```rust
    fn winning_position(&self) -> Bitboard {
        compute_winning_position(self.current_position, self.mask)
    }

    fn opponent_winning_position(&self) -> Bitboard {
        compute_winning_position(self.current_position ^ self.mask, self.mask)
    }
}
```

These helpers call the shared winning-cell computation for the player to move
and for the opponent. The implementation block closes after these helpers.

### `Display` For `PlayColumnError`

```rust
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
```

The displayed column numbers are 1-based for readers, even though the stored
columns are zero-based.

### Error Trait For `PlayColumnError`

```rust
impl Error for PlayColumnError {}
```

This empty implementation lets `PlayColumnError` participate in normal Rust
error handling.

### `Display` For `PlaySequenceError`

```rust
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
```

The outer sequence error adds human-facing move numbering and delegates the
reason text to `PlaySequenceErrorKind`.

### Error Trait For `PlaySequenceError`

```rust
impl Error for PlaySequenceError {}
```

This marks the sequence error as a standard Rust error.

### `Display` For `PlaySequenceErrorKind`

```rust
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
```

Each error kind gets a concise message. The character cases report the original
character. The column cases convert zero-based columns to 1-based text.

### Error Trait For `PlaySequenceErrorKind`

```rust
impl Error for PlaySequenceErrorKind {}
```

This keeps the inner error kind usable on its own.

### `column_mask`

```rust
#[must_use]
pub const fn column_mask(column: usize) -> Bitboard {
    assert!(column < WIDTH);
    ((1_u64 << HEIGHT) - 1) << (column * COLUMN_STRIDE)
}
```

`column_mask` returns all playable cells in one column. The `assert!` makes
out-of-range calls fail immediately.

### `top_mask_col`

```rust
#[must_use]
pub const fn top_mask_col(column: usize) -> Bitboard {
    assert!(column < WIDTH);
    1_u64 << ((HEIGHT - 1) + column * COLUMN_STRIDE)
}
```

`top_mask_col` returns the top playable cell, not the sentinel bit.

### `bottom_mask_col`

```rust
#[must_use]
pub const fn bottom_mask_col(column: usize) -> Bitboard {
    assert!(column < WIDTH);
    1_u64 << (column * COLUMN_STRIDE)
}
```

`bottom_mask_col` returns the bottom playable cell in one column.

### `bottom_mask`

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

`bottom_mask` builds `BOTTOM_MASK` in a `const fn` loop so the global mask is
available at compile time.

### `parse_column_character`

```rust
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
```

This parser accepts only ASCII digits. It returns a zero-based column for valid
digits and preserves the original character in any parse error.

### `compute_winning_position`

```rust
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
```

The function computes empty cells where `position` would complete four in a
row. The first expression handles vertical wins. The three `pairs` groups
handle horizontal, diagonal-up, and diagonal-down patterns. The final mask keeps
only empty playable cells.

## Tests

### Test Module Header

```rust
#[cfg(test)]
mod tests {
    use super::{
        BOARD_MASK, BOTTOM_MASK, Bitboard, HEIGHT, PlayColumnError, PlaySequenceError,
        PlaySequenceErrorKind, Position, WIDTH, bottom_mask_col, column_mask, top_mask_col,
    };
```

The test module compiles only during tests. It imports private and public items
from the parent module so tests can inspect internal bitboards directly.

### `masks_match_the_documented_bit_layout`

```rust
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
```

This test verifies the public mask helpers against the documented bit layout.
It checks bottom bits, top playable bits, full column masks, and membership in
`BOARD_MASK`.

### `plays_short_sequence_and_tracks_state`

```rust
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
```

This test parses a small position and checks the exact internal bitboards. It
also confirms that a partially filled column remains playable.

### `rejects_bad_sequence_characters_and_columns`

```rust
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
```

The parser distinguishes a non-digit from an out-of-range digit and preserves
the failing move index.

### `rejects_full_columns_and_winning_moves_in_sequences`

```rust
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
```

The sequence parser rejects a seventh disc in a column and rejects a move that
would already finish the game.

### `play_col_checks_bounds_and_full_columns`

```rust
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
```

Direct column play reports out-of-range columns before full-column checks. The
loop fills column 0, after which `can_play` and `play_col` agree that the
column is unavailable.

### `accepts_a_full_board_draw_sequence`

```rust
    #[test]
    fn accepts_a_full_board_draw_sequence() {
        let draw = "177322644317353514472267227353611516544566";
        let position = Position::from_sequence(draw).unwrap();
        assert_eq!(position.move_count(), WIDTH * HEIGHT);
        assert_eq!(position.possible(), 0);
    }
```

This test uses the manual full-board draw fixture. It checks that all 42 moves
parse and that no legal moves remain.

### `detects_immediate_wins`

```rust
    #[test]
    fn detects_immediate_wins() {
        let position = Position::from_sequence("121212").unwrap();
        assert!(position.can_win_next());
        assert!(position.is_winning_move(0));
        assert!(!position.is_winning_move(1));
    }
```

The position gives the player to move an immediate vertical win in column 0.
The neighboring column is playable but not winning.

### `filters_non_losing_moves_to_forced_block`

```rust
    #[test]
    fn filters_non_losing_moves_to_forced_block() {
        let position = Position::from_sequence("12121").unwrap();
        assert!(!position.can_win_next());
        assert_eq!(position.possible_non_losing_moves(), bit(0, 3));
    }
```

This position requires a block in column 0. The non-losing move filter returns
only that move bit.

### `symmetric_positions_have_the_same_key3`

```rust
    #[test]
    fn symmetric_positions_have_the_same_key3() {
        let position = Position::from_sequence("4453").unwrap();
        let mirrored = Position::from_sequence("4435").unwrap();
        assert_eq!(position.key3(), mirrored.key3());
    }
```

`key3` canonicalizes mirrored positions, so these two positions share the same
opening-book key.

### `selected_benchmark_positions_parse`

```rust
    #[test]
    fn selected_benchmark_positions_parse() {
        for sequence in include_str!("../tests/fixtures/positions/Seq_L1_R1")
            .lines()
            .take(32)
        {
            Position::from_sequence(sequence).unwrap();
        }
    }
```

This test connects `Position` parsing to the tracked fixture data without
running the full solver.

### `bit`

```rust
    fn bit(column: usize, row: usize) -> Bitboard {
        1_u64 << (row + column * (HEIGHT + 1))
    }
```

The test helper computes one documented cell bit from a column and row.

### `assert_sequence_error`

```rust
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
```

This helper keeps parser-error tests compact. It checks both the failing move
index and the precise error kind, then closes the test module.

## Important Invariants

- `WIDTH`, `HEIGHT`, and `COLUMN_STRIDE` define the bit layout used throughout
  the solver.
- `BOARD_MASK` must contain only playable cells and no sentinel bits.
- `current_position` must always be stored from the perspective of the player
  to move.
- Parsed sequences represent non-terminal positions.
- `possible_non_losing_moves` assumes `can_win_next` is false.
- `key` and `key3` must remain stable because later tables and books depend on
  them.

## Algorithm Context

This file supplies the low-level board operations that make exact search
practical. The solver will repeatedly copy positions, generate legal moves,
reject immediately losing moves, score candidate moves, and look positions up
by key. All of those operations depend on the bitboard representation defined
here.

## Extension Notes

Keep public methods semantic where possible. A future caller usually needs to
ask whether a move is legal, whether it wins, or what key a position has; it
usually should not need direct mutable access to the internal bitboards.

If support for non-7x6 boards is added later, revisit the board constants,
score bounds, opening-book assumptions, and `u64` storage together.
