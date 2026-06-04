# Bitboards

The solver stores a Connect4 position in two `u64` bitboards. This follows
Pascal Pons' final C++ representation and the bitboard tutorial at
`http://blog.gamesolver.org/solving-connect-four/06-bitboard/`.

## Layout

The standard board has 7 columns and 6 playable rows. Each column gets one
extra sentinel bit, so a column uses 7 bits and the whole board uses 49 bits.

The bit numbers are:

```text
.  .  .  .  .  .  .
5 12 19 26 33 40 47
4 11 18 25 32 39 46
3 10 17 24 31 38 45
2  9 16 23 30 37 44
1  8 15 22 29 36 43
0  7 14 21 28 35 42
```

The top row shown as dots is the sentinel row. It is not playable, but it makes
full columns and move generation cheap to detect.

In Rust, the main constants are:

```rust
pub const WIDTH: usize = 7;
pub const HEIGHT: usize = 6;
pub const COLUMN_STRIDE: usize = HEIGHT + 1;
```

## Two Boards

`Position` stores:

- `mask`: every occupied cell,
- `current_position`: occupied cells belonging to the side to move,
- `moves`: the number of played moves.

After every move, `current_position ^= mask` switches the perspective from the
player who just moved to the next player. Then the new move bit is added to
`mask`.

This representation is compact, but the main benefit is speed: checking moves,
creating transposition keys, and finding winning cells are all bit operations.

## Masks

`BOTTOM_MASK` has one bit in the bottom cell of each column:

```text
bits 0, 7, 14, 21, 28, 35, 42
```

`BOARD_MASK` contains every playable cell and no sentinel bits. The helper
functions `column_mask`, `bottom_mask_col`, and `top_mask_col` name the common
column-local masks used by the solver.

The legal move bitboard is:

```rust
(mask + BOTTOM_MASK) & BOARD_MASK
```

Adding `BOTTOM_MASK` carries each column up to its first empty cell. Masking
with `BOARD_MASK` removes sentinel bits and full columns.

## Keys

The ordinary transposition-table key is:

```rust
current_position + mask
```

The addition reconstructs Pons' compact position encoding, where the first
empty cell in each column acts as a marker.

The opening book uses `key3`, a base-3 representation that maps mirrored
positions to the same value. `Position::key3` builds the key from left to right
and right to left, then keeps the smaller representation.

## Winning Cells

The solver does not only ask whether a player has already connected four. It
often asks which empty cells would complete four. `compute_winning_position`
uses shifts for the four directions:

- vertical: `1`,
- horizontal: `HEIGHT + 1`,
- diagonal up: `HEIGHT`,
- diagonal down: `HEIGHT + 2`.

The result is masked by `BOARD_MASK ^ occupied_mask`, so only empty playable
cells remain.

This is why functions such as `is_winning_move`, `can_win_next`,
`possible_non_losing_moves`, and `move_score` can be implemented without
scanning the board cell by cell.

## Implemented In This Repo

The bitboard representation is implemented in `src/position.rs`.

The main code-level pieces are:

- `Bitboard` is a type alias for `u64`.
- `WIDTH`, `HEIGHT`, `BOARD_SIZE`, and `COLUMN_STRIDE` define the board shape.
- `BOTTOM_MASK`, `BOARD_MASK`, `column_mask`, `bottom_mask_col`, and
  `top_mask_col` define the masks used throughout move generation.
- `Position` stores `current_position`, `mask`, and `moves`.
- `Position::possible` computes all legal move bits with
  `(mask + BOTTOM_MASK) & BOARD_MASK`.
- `Position::play_bitboard` applies one already-computed move bit and switches
  the side-to-move perspective.
- `Position::key` builds the transposition-table key.
- `Position::key3` builds the mirrored canonical opening-book key.
- `compute_winning_position` implements all four alignment directions with
  shifts and masks.

The source note [notes/src/position.rs.md](../src/position.rs.md) walks through
these pieces in source order. The solver note
[notes/src/solver.rs.md](../src/solver.rs.md) shows how the search consumes
the bitboard operations.
