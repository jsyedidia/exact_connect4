# src/move_sorter.rs

## Role In The System

`src/move_sorter.rs` defines the small move-ordering helper used by the search.
Connect4 has at most seven legal moves, so the sorter uses a fixed array and
insertion sorting instead of heap allocation or a general-purpose collection.

The sorter stores moves in increasing score order. `pop` removes from the end
of the active range, so callers receive the highest-scoring move first.

Related concept notes:

- [notes/concepts/move_ordering.md](../concepts/move_ordering.md)
- [notes/concepts/alpha_beta.md](../concepts/alpha_beta.md)

## Code Walkthrough

### Imports

```rust
use crate::position::{Bitboard, WIDTH};
```

The sorter stores moves as `Bitboard` values and uses `WIDTH` as its fixed
capacity.

### Entry

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Entry {
    move_bit: Bitboard,
    score: i32,
}
```

Each array slot stores one playable move bit and the score used to order it.
`Entry` is private because callers only need to add and pop moves.

### MoveSorter

```rust
#[derive(Debug, Clone)]
pub struct MoveSorter {
    entries: [Entry; WIDTH],
    len: usize,
}
```

`entries` is always seven slots long. `len` marks how many of those slots are
currently active.

### Constructor

```rust
impl MoveSorter {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            entries: [Entry {
                move_bit: 0,
                score: 0,
            }; WIDTH],
            len: 0,
        }
    }
```

The constructor fills the fixed array with harmless default entries and sets
the active length to zero.

### Adding A Move

```rust
    pub fn add(&mut self, move_bit: Bitboard, score: i32) {
        assert!(self.len < WIDTH, "move sorter capacity exceeded");

        let mut position = self.len;
        self.len += 1;

        while position > 0 && self.entries[position - 1].score > score {
            self.entries[position] = self.entries[position - 1];
            position -= 1;
        }

        self.entries[position] = Entry { move_bit, score };
    }
```

`add` first reserves one new active slot. It then shifts larger scores one slot
to the right until it finds the insertion point for the new score. Equal scores
are not shifted, which means a later move with the same score is placed after
earlier equal-score moves and will be popped first.

### Popping A Move

```rust
    pub fn pop(&mut self) -> Option<Bitboard> {
        if self.len == 0 {
            return None;
        }

        self.len -= 1;
        Some(self.entries[self.len].move_bit)
    }
```

An empty sorter returns `None`. Otherwise, `pop` shortens the active range and
returns the move that used to be the last active entry.

### Default

```rust
}

impl Default for MoveSorter {
    fn default() -> Self {
        Self::new()
    }
}
```

`Default` lets callers create an empty sorter with the standard Rust
convention.

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::MoveSorter;
    use crate::position::WIDTH;

    #[test]
    fn pops_moves_from_highest_score_to_lowest_score() {
        let mut sorter = MoveSorter::new();

        sorter.add(10, 4);
        sorter.add(20, 1);
        sorter.add(30, 7);
        sorter.add(40, 3);

        assert_eq!(sorter.pop(), Some(30));
        assert_eq!(sorter.pop(), Some(10));
        assert_eq!(sorter.pop(), Some(40));
        assert_eq!(sorter.pop(), Some(20));
        assert_eq!(sorter.pop(), None);
    }

    #[test]
    fn equal_scores_pop_newest_first() {
        let mut sorter = MoveSorter::new();

        sorter.add(10, 5);
        sorter.add(20, 5);
        sorter.add(30, 5);

        assert_eq!(sorter.pop(), Some(30));
        assert_eq!(sorter.pop(), Some(20));
        assert_eq!(sorter.pop(), Some(10));
    }

    #[test]
    #[should_panic(expected = "move sorter capacity exceeded")]
    fn panics_when_more_than_width_moves_are_added() {
        let mut sorter = MoveSorter::new();

        for index in 0..=WIDTH {
            sorter.add(index as u64 + 1, index as i32);
        }
    }
}
```

The tests cover score ordering, equal-score tie behavior, empty pop behavior,
and the fixed-capacity assertion.
