# Transposition Tables

A transposition table is a cache for positions already seen during search.
Connect4 move orders can reach the same board through different sequences, and
alpha-beta search repeatedly asks for bounds on related positions. Caching
known results avoids redoing some of that work.

The table in `src/transposition_table.rs` is deliberately simple. A full
position key maps to one slot by taking `key % capacity`. The slot stores a
partial key and a compact value. On lookup, the table checks whether the stored
partial key matches the requested key; if it does, the stored value is returned.
If it does not, the lookup is a miss.

There are two kinds of collision to keep in mind:

- Index collision: two full keys map to the same slot. The newer entry replaces
  the older one.
- Partial-key collision: two full keys map to the same slot and have the same
  truncated key fragment. The table may return the slot value for either key.

Partial-key collisions are possible by design. The benefit is that the table can
store many compact entries and keep lookups extremely cheap. The solver's score
encoding and search logic must therefore use the table as a performance cache,
not as the sole source of truth.

The table reserves the default value, usually zero, to mean "missing." Stored
solver values are encoded as nonzero values before insertion.

The capacity is chosen as the next prime greater than or equal to `2^LOG_SIZE`.
Using a prime capacity helps spread modulo indices for structured bitboard keys.

## Implemented In This Repo

The reusable table is implemented in `src/transposition_table.rs`; the solver
uses it from `src/solver.rs`.

The main code-level pieces are:

- `PartialKey` defines how a full `u64` key is truncated into a slot key.
- `TranspositionTable<K, V, LOG_SIZE>` owns parallel `Vec<K>` and `Vec<V>`
  arrays for keys and values.
- `TranspositionTable::new` allocates a prime-sized table based on
  `table_size::<LOG_SIZE>()`.
- `put` overwrites the one slot selected by the full key.
- `get_raw` returns the stored value when the partial key matches, or the
  default value on a miss.
- `get` wraps `get_raw` in `Option` for call sites that prefer explicit
  `Some`/`None`.
- `reset` clears both arrays back to the default value.
- `next_prime`, `table_size`, and `log2_floor` support both the search table
  and opening-book table sizing.
- `SolverWithTable` uses `TranspositionTable<u32, u8, LOG_SIZE>` for compact
  alpha-beta bounds.
- `OpeningBook` reuses `next_prime` so book lookup uses the same prime-capacity
  convention as the table format.

The matching source notes are
[notes/src/transposition_table.rs.md](../src/transposition_table.rs.md),
[notes/src/solver.rs.md](../src/solver.rs.md), and
[notes/src/opening_book.rs.md](../src/opening_book.rs.md).
