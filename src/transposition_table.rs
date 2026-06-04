// SPDX-FileCopyrightText: 2017-2019 Pascal Pons <contact@gamesolver.org>
// SPDX-FileCopyrightText: 2026 Jonathan Yedidia
// SPDX-License-Identifier: AGPL-3.0-or-later

/// Integer type that can store the partial key kept in a table slot.
pub trait PartialKey: Copy + Default + Eq {
    /// Truncates a full position key to the key fragment stored in the table.
    fn from_key(key: u64) -> Self;
}

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

/// Fixed-size transposition table with one partial key and one value per slot.
#[derive(Debug, Clone)]
pub struct TranspositionTable<K, V, const LOG_SIZE: usize>
where
    K: PartialKey,
    V: Copy + Default + Eq,
{
    keys: Vec<K>,
    values: Vec<V>,
}

impl<K, V, const LOG_SIZE: usize> TranspositionTable<K, V, LOG_SIZE>
where
    K: PartialKey,
    V: Copy + Default + Eq,
{
    /// Creates an empty table with a prime capacity near `2^LOG_SIZE`.
    #[must_use]
    pub fn new() -> Self {
        let capacity = table_size::<LOG_SIZE>();
        Self {
            keys: vec![K::default(); capacity],
            values: vec![V::default(); capacity],
        }
    }

    /// Returns the number of slots in the table.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.keys.len()
    }

    /// Stores a nonzero value for a position key.
    pub fn put(&mut self, key: u64, value: V) {
        debug_assert!(value != V::default(), "stored table values must be nonzero");

        let index = self.index(key);
        self.keys[index] = K::from_key(key);
        self.values[index] = value;
    }

    /// Looks up a position key.
    ///
    /// A matching partial key with the default value is treated as missing.
    #[must_use]
    pub fn get(&self, key: u64) -> Option<V> {
        let value = self.get_raw(key);
        if value == V::default() {
            None
        } else {
            Some(value)
        }
    }

    /// Looks up a position key and returns the default value on a miss.
    #[must_use]
    pub fn get_raw(&self, key: u64) -> V {
        let index = self.index(key);
        if self.keys[index] == K::from_key(key) {
            self.values[index]
        } else {
            V::default()
        }
    }

    /// Clears all table slots.
    pub fn reset(&mut self) {
        self.keys.fill(K::default());
        self.values.fill(V::default());
    }

    fn index(&self, key: u64) -> usize {
        key as usize % self.capacity()
    }
}

impl<K, V, const LOG_SIZE: usize> Default for TranspositionTable<K, V, LOG_SIZE>
where
    K: PartialKey,
    V: Copy + Default + Eq,
{
    fn default() -> Self {
        Self::new()
    }
}

/// Returns the prime table size used for a requested log size.
#[must_use]
pub const fn table_size<const LOG_SIZE: usize>() -> usize {
    assert!(LOG_SIZE < u64::BITS as usize);
    next_prime(1_u64 << LOG_SIZE) as usize
}

/// Returns the smallest prime greater than or equal to `value`.
#[must_use]
pub const fn next_prime(value: u64) -> u64 {
    let mut candidate = if value < 2 { 2 } else { value };
    while !is_prime(candidate) {
        candidate += 1;
    }
    candidate
}

/// Returns the floor of `log2(value)`, with `log2_floor(0) == 0`.
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
