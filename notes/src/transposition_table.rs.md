# src/transposition_table.rs

## Role In The System

`src/transposition_table.rs` defines the fixed-size cache used by the search to
remember previously evaluated positions. Each slot stores a truncated position
key and a compact value. The table is intentionally simple: one computed index,
one slot, no chaining, and replacement on collision.

The table treats the default value as "missing." For score encodings this means
stored entries must be nonzero.

Related concept notes:

- [notes/concepts/transposition_tables.md](../concepts/transposition_tables.md)
- [notes/concepts/alpha_beta.md](../concepts/alpha_beta.md)

## Code Walkthrough

### PartialKey

```rust
pub trait PartialKey: Copy + Default + Eq {
    fn from_key(key: u64) -> Self;
}
```

`PartialKey` names the integer types that can store the key fragment held in a
table slot. The bounds are the operations the table needs: copy the key, create
the empty key value, and compare stored keys.

```rust
impl PartialKey for u8 {
    fn from_key(key: u64) -> Self {
        key as Self
    }
}

impl PartialKey for u16 {
    fn from_key(key: u64) -> Self {
        key as Self
    }
}

impl PartialKey for u32 {
    fn from_key(key: u64) -> Self {
        key as Self
    }
}

impl PartialKey for u64 {
    fn from_key(key: u64) -> Self {
        key
    }
}
```

The smaller integer implementations intentionally truncate the full key. The
`u64` implementation keeps the complete key.

### TranspositionTable

```rust
#[derive(Debug, Clone)]
pub struct TranspositionTable<K, V, const LOG_SIZE: usize>
where
    K: PartialKey,
    V: Copy + Default + Eq,
{
    keys: Vec<K>,
    values: Vec<V>,
}
```

The table is generic over the partial key type, value type, and requested log
size. Keys and values live in separate vectors so each slot has one key fragment
and one value at the same index.

```rust
impl<K, V, const LOG_SIZE: usize> TranspositionTable<K, V, LOG_SIZE>
where
    K: PartialKey,
    V: Copy + Default + Eq,
{
```

The implementation repeats the same bounds because every method depends on
those type capabilities.

### Constructor And Capacity

```rust
    #[must_use]
    pub fn new() -> Self {
        let capacity = table_size::<LOG_SIZE>();
        Self {
            keys: vec![K::default(); capacity],
            values: vec![V::default(); capacity],
        }
    }

    #[must_use]
    pub fn capacity(&self) -> usize {
        self.keys.len()
    }
```

`new` rounds the requested power-of-two size up to a prime table capacity and
fills both vectors with default values. `capacity` reports the number of slots.

### Writing Entries

```rust
    pub fn put(&mut self, key: u64, value: V) {
        debug_assert!(value != V::default(), "stored table values must be nonzero");

        let index = self.index(key);
        self.keys[index] = K::from_key(key);
        self.values[index] = value;
    }
```

`put` computes the slot index, stores the truncated key, and stores the value.
Any previous entry at that index is replaced.

### Reading Entries

```rust
    #[must_use]
    pub fn get(&self, key: u64) -> Option<V> {
        let value = self.get_raw(key);
        if value == V::default() {
            None
        } else {
            Some(value)
        }
    }

    #[must_use]
    pub fn get_raw(&self, key: u64) -> V {
        let index = self.index(key);
        if self.keys[index] == K::from_key(key) {
            self.values[index]
        } else {
            V::default()
        }
    }
```

`get_raw` returns the stored value when the partial key matches and the default
value otherwise. `get` wraps that convention in `Option`, which makes missing
entries explicit at call sites.

### Reset And Indexing

```rust
    pub fn reset(&mut self) {
        self.keys.fill(K::default());
        self.values.fill(V::default());
    }

    fn index(&self, key: u64) -> usize {
        key as usize % self.capacity()
    }
}
```

`reset` clears every slot. `index` maps a full key to one table slot by taking
the key modulo the prime capacity.

### Default

```rust
impl<K, V, const LOG_SIZE: usize> Default for TranspositionTable<K, V, LOG_SIZE>
where
    K: PartialKey,
    V: Copy + Default + Eq,
{
    fn default() -> Self {
        Self::new()
    }
}
```

`Default` delegates to the normal constructor.

### Table Sizing

```rust
#[must_use]
pub const fn table_size<const LOG_SIZE: usize>() -> usize {
    assert!(LOG_SIZE < u64::BITS as usize);
    next_prime(1_u64 << LOG_SIZE) as usize
}
```

`table_size` computes a prime capacity at least as large as `2^LOG_SIZE`.

```rust
#[must_use]
pub const fn next_prime(value: u64) -> u64 {
    let mut candidate = if value < 2 { 2 } else { value };
    while !is_prime(candidate) {
        candidate += 1;
    }
    candidate
}
```

`next_prime` walks upward from the requested value until it reaches a prime.

```rust
#[must_use]
pub const fn log2_floor(value: u32) -> u32 {
    if value <= 1 {
        return 0;
    }

    let mut result = 0;
    let mut remaining = value;
    while remaining > 1 {
        remaining /= 2;
        result += 1;
    }
    result
}
```

`log2_floor` counts how many times the value can be divided by two before it
reaches one.

```rust
const fn is_prime(value: u64) -> bool {
    if value < 2 {
        return false;
    }
    if value == 2 {
        return true;
    }
    if value.is_multiple_of(2) {
        return false;
    }

    let mut factor = 3;
    while factor <= value / factor {
        if value.is_multiple_of(factor) {
            return false;
        }
        factor += 2;
    }
    true
}
```

`is_prime` handles small values, rejects even composites, and then tests odd
factors up to the square root.

### Tests

```rust
#[cfg(test)]
mod tests {
    use super::{TranspositionTable, log2_floor, next_prime, table_size};

    #[test]
    fn prime_sizing_matches_expected_values() {
        assert_eq!(next_prime(0), 2);
        assert_eq!(next_prime(2), 2);
        assert_eq!(next_prime(8), 11);
        assert_eq!(next_prime(1 << 20), 1_048_583);
        assert_eq!(table_size::<1>(), 2);
        assert_eq!(table_size::<8>(), 257);
        assert_eq!(table_size::<20>(), 1_048_583);
        assert_eq!(table_size::<21>(), 2_097_169);
        assert_eq!(table_size::<22>(), 4_194_319);
        assert_eq!(table_size::<23>(), 8_388_617);
        assert_eq!(table_size::<24>(), 16_777_259);
        assert_eq!(table_size::<25>(), 33_554_467);
        assert_eq!(table_size::<26>(), 67_108_879);
        assert_eq!(table_size::<27>(), 134_217_757);
    }

    #[test]
    fn log2_floor_handles_small_and_large_values() {
        assert_eq!(log2_floor(0), 0);
        assert_eq!(log2_floor(1), 0);
        assert_eq!(log2_floor(2), 1);
        assert_eq!(log2_floor(3), 1);
        assert_eq!(log2_floor(4), 2);
        assert_eq!(log2_floor(1023), 9);
        assert_eq!(log2_floor(1024), 10);
    }

    #[test]
    fn put_and_get_round_trip_values() {
        let mut table = TranspositionTable::<u16, u8, 4>::new();

        table.put(42, 9);

        assert_eq!(table.get(42), Some(9));
        assert_eq!(table.get_raw(42), 9);
        assert_eq!(table.get(43), None);
        assert_eq!(table.get_raw(43), 0);
    }

    #[test]
    fn overwriting_a_slot_replaces_the_previous_value() {
        let mut table = TranspositionTable::<u16, u8, 4>::new();

        table.put(42, 9);
        table.put(42, 11);

        assert_eq!(table.get(42), Some(11));
    }

    #[test]
    fn index_collisions_keep_only_the_latest_matching_key() {
        let mut table = TranspositionTable::<u16, u8, 4>::new();
        let colliding_key = 42 + table.capacity() as u64;

        table.put(42, 9);
        table.put(colliding_key, 11);

        assert_eq!(table.get(colliding_key), Some(11));
        assert_eq!(table.get(42), None);
    }

    #[test]
    fn partial_key_collisions_can_return_the_slot_value() {
        let mut table = TranspositionTable::<u8, u8, 4>::new();
        let key = 42;
        let colliding_partial_key = key + 256 * table.capacity() as u64;

        table.put(key, 9);
        table.put(colliding_partial_key, 11);

        assert_eq!(table.get(key), Some(11));
        assert_eq!(table.get(colliding_partial_key), Some(11));
    }

    #[test]
    fn reset_clears_stored_entries() {
        let mut table = TranspositionTable::<u16, u8, 4>::new();

        table.put(42, 9);
        table.reset();

        assert_eq!(table.get(42), None);
        assert_eq!(table.get_raw(42), 0);
    }
}
```

The tests cover sizing, lookup, overwriting, index collisions, partial-key
collisions, and reset behavior.
