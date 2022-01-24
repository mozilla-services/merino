#![warn(missing_docs)]

//! A data structure that can efficiently store many keys that map to a
//! relatively small number of values.

use dashmap::{mapref::entry::Entry, DashMap};
use std::collections::hash_map::DefaultHasher;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

/// A hashmap that assumes a large number of keys will map to a relatively smaller number of values.
///
/// If a value is stored in the map that is already stored by another key, the
/// refcount of the value will be incremented instead of duplicating it.
///
/// Uses DashMap internally
#[derive(Debug)]
pub struct DedupedMap<K, M, V>
where
    K: Eq + Hash,
    M: Debug,
    V: Debug,
{
    /// First layer of the map. A mapping from incoming requests to a u64 pointer
    /// into the cache storage.
    pointers: DashMap<K, MapPointer<M>>,

    /// Second layer of the map. The items stored in the cache, keyed by their
    /// hash.
    storage: DashMap<u64, MapValue<V>>,
}

/// The first layer of the map, it stores per-key metadata, and a hash entry for the second layer.
#[derive(Debug)]
struct MapPointer<M: Debug> {
    /// The metadata associated with this pointer.
    meta: M,
    /// The hash of the content to retrieve from the storage.
    hash: u64,
}

impl<M: Clone + Debug> Clone for MapPointer<M> {
    fn clone(&self) -> Self {
        Self {
            meta: self.meta.clone(),
            hash: self.hash,
        }
    }
}

/// The second layer of the map, a reference counted value.
#[derive(Debug)]
struct MapValue<V> {
    /// The stored value.
    value: V,
    /// The number of pointers that are referring to this storage item.
    refcount: usize,
}

impl<V: PartialEq> PartialEq for MapValue<V> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.refcount == other.refcount
    }
}

impl<V: Clone> Clone for MapValue<V> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            refcount: self.refcount,
        }
    }
}

impl<K, M, V> DedupedMap<K, M, V>
where
    K: Eq + Hash + Debug,
    M: Debug + Clone,
    V: Hash + Debug + Clone,
{
    /// Create an empty map.
    #[must_use]
    pub fn new() -> Self {
        Self {
            storage: DashMap::new(),
            pointers: DashMap::new(),
        }
    }

    /// Insert `value` into the map at `key` with metadata `meta`.
    ///
    /// If `value` is already in the map under a different key, its refcount will
    /// be incremented instead of storing another copy of `value`. The metadata
    /// data `meta` will be attached to this specific key, and not refcounted.
    pub fn insert(&self, key: K, meta: M, value: V) {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        let hash = hasher.finish();

        // This order reduces the chance of seeing a dangling pointer, since we
        // wont have a time where there is a pointer pointing to nothing. It
        // increases the chances of orphaned storage entries. An orphan is safer
        // than a dangling pointer, as an orphan is similar to a memory leak,
        // instead of allowing for potential race conditions.
        match self.storage.entry(hash) {
            Entry::Occupied(mut occupied_entry) => {
                occupied_entry.get_mut().refcount += 1;
            }
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(MapValue { value, refcount: 1 });
            }
        }

        self.pointers.insert(key, MapPointer { meta, hash });
    }

    /// Remove the item associated with a key from the map.
    ///
    /// The metadata associated with `key` will always be removed. The reference
    /// count of the storage item it points to will be decremented. If no more
    /// keys refer to the storage item, it will also be removed.
    pub fn remove(&self, key: K) {
        let key_desc = format!("{:?}", key);
        if let Entry::Occupied(occupied_pointer_entry) = self.pointers.entry(key) {
            let pointer = occupied_pointer_entry.remove();
            match self.storage.entry(pointer.hash) {
                Entry::Occupied(mut occupied_storage_entry) => {
                    let item = occupied_storage_entry.get_mut();
                    if item.refcount > 1 {
                        item.refcount -= 1;
                    } else {
                        occupied_storage_entry.remove();
                    }
                }
                Entry::Vacant(_) => {
                    tracing::error!(
                        r#type = "deduped-dashmap.remove.dangling-entry",
                        key = %key_desc, "Dangling storage entry");
                }
            }
        }
    }

    /// Get cloned copies of the metadata and value associated with `key`.
    pub fn get(&self, key: &K) -> Option<(M, V)> {
        match self.pointers.get(key) {
            Some(pointer_ref) => {
                if let Some(storage_ref) = self.storage.get(&pointer_ref.hash) {
                    let meta = pointer_ref.meta.clone();
                    let value = storage_ref.value.clone();
                    Some((meta, value))
                } else {
                    tracing::error!(
                        r#type = "deduped-dashmap.get.dangling-entry",
                        ?key,
                        "Dangling storage entry"
                    );
                    None
                }
            }
            None => None,
        }
    }

    /// Fetches the total number of storage items in the map.
    ///
    /// This will be at most `self.len_pointers()`.
    #[must_use]
    pub fn len_storage(&self) -> usize {
        self.storage.len()
    }

    /// Fetches the total number of pointers stored in the map.
    ///
    /// This will be at least `self.len_storage()`.
    #[must_use]
    pub fn len_pointers(&self) -> usize {
        self.pointers.len()
    }

    /// Retain elements based on the result of a predicate.
    ///
    /// If the predicate function returns:
    ///
    /// - `ControlFlow::Continue(true)`: The item will be retained and the iteration will continue.
    /// - `ControlFlow::Continue(false)`: The item will be removed and the iteration will continue.
    /// - `ControlFlow::Break(())`: This item, and all further items in the map,
    ///   will be retained, and the predicate won't be called again.
    pub fn retain<F>(&self, mut pred: F)
    where
        F: FnMut(&K, &M, &V) -> ControlFlow<bool>,
    {
        let mut should_continue = true;

        self.pointers.retain(|key, pointer| -> bool {
            // it would be nice if we could stop the `dashmap`'s iteration, but we can't.
            if !should_continue {
                return true;
            }

            match self.storage.entry(pointer.hash) {
                Entry::Occupied(mut occupied_entry) => {
                    let item = occupied_entry.get_mut();
                    let should_keep = pred(key, &pointer.meta, &item.value);

                    match should_keep {
                        ControlFlow::Continue(true) => true,
                        ControlFlow::Continue(false) => {
                            if item.refcount > 1 {
                                item.refcount -= 1;
                            } else {
                                occupied_entry.remove();
                            }
                            false
                        }
                        ControlFlow::Break => {
                            should_continue = false;
                            true
                        }
                    }
                }
                Entry::Vacant(_) => {
                    tracing::error!(
                        r#type = "deduped-dashmap.retain.missing-entry",
                        "missing storage entry in memory cache"
                    );
                    false
                }
            }
        });
    }

    /// Checks if the map contains a specific key.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn contains_key(&self, key: &K) -> bool {
        self.pointers.contains_key(key)
    }
}

impl<K, M, V> Default for DedupedMap<K, M, V>
where
    K: Eq + Hash,
    M: Debug,
    V: Debug,
{
    fn default() -> Self {
        Self {
            pointers: DashMap::default(),
            storage: DashMap::default(),
        }
    }
}

impl<K, M, V> FromIterator<(K, M, V)> for DedupedMap<K, M, V>
where
    K: Debug + Eq + Hash,
    M: Clone + Debug,
    V: Clone + Debug + Eq + Hash,
{
    fn from_iter<T: IntoIterator<Item = (K, M, V)>>(iter: T) -> Self {
        let map = Self::new();
        for (k, m, v) in iter {
            map.insert(k, m, v);
        }
        map
    }
}

impl<K, V> FromIterator<(K, V)> for DedupedMap<K, (), V>
where
    K: Debug + Eq + Hash,
    V: Clone + Debug + Eq + Hash,
{
    fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
        iter.into_iter().map(|(k, v)| (k, (), v)).collect()
    }
}

/// This mimic's the unstable API std::ops::ControlFlow, except the Break variant here doesn't have a value.
#[derive(Debug)]
pub enum ControlFlow<C> {
    /// Continue to the next iteration.
    Continue(C),
    /// Stop after this iteration.
    Break,
}

#[cfg(test)]
mod tests {
    use crate::MapValue;

    use super::{ControlFlow, DedupedMap};
    use std::collections::{HashMap, HashSet};

    #[test]
    fn test_simple() {
        // Set up a map with some values related to days of the week
        let map = DedupedMap::<&str, usize, &str>::new();
        map.insert("monday", 0, "week day");
        map.insert("tuesday", 1, "week day");
        map.insert("wednesday", 2, "week day");
        map.insert("thursday", 3, "week day");
        map.insert("friday", 4, "week day");
        map.insert("saturday", 5, "week end");
        map.insert("sunday", 6, "week end");

        // Make some assertions about the initial state
        assert_eq!(map.len_pointers(), 7);
        assert_eq!(map.len_storage(), 2);

        let mut pointers = map.pointers.clone().into_iter().collect::<Vec<_>>();
        pointers.sort_by_key(|v| v.0);
        let storage = map.storage.clone().into_iter().collect::<HashMap<_, _>>();

        let cases = vec![
            ("friday", 4, "week day", 5),
            ("monday", 0, "week day", 5),
            ("saturday", 5, "week end", 2),
            ("sunday", 6, "week end", 2),
            ("thursday", 3, "week day", 5),
            ("tuesday", 1, "week day", 5),
            ("wednesday", 2, "week day", 5),
        ];
        for (idx, (day, day_num, day_type, refcount)) in cases.into_iter().enumerate() {
            assert_eq!(pointers[idx].0, day);
            assert_eq!(pointers[idx].1.meta, day_num);
            assert_eq!(
                storage[&pointers[idx].1.hash],
                MapValue {
                    value: day_type,
                    refcount,
                }
            );
        }

        // Remove some items from the map
        map.remove("sunday");
        assert_eq!(map.len_pointers(), 6);
        assert_eq!(map.len_storage(), 2);
        map.remove("saturday");
        assert_eq!(map.len_pointers(), 5);
        assert_eq!(map.len_storage(), 1);
        map.remove("monday");
        assert_eq!(map.len_pointers(), 4);
        assert_eq!(map.len_storage(), 1);

        // Make sure the internal structure changed as expected
        let mut pointers = map.pointers.clone().into_iter().collect::<Vec<_>>();
        pointers.sort_by_key(|v| v.0);
        let storage = map.storage.clone().into_iter().collect::<HashMap<_, _>>();

        let cases = vec![
            ("friday", 4, "week day", 4),
            ("thursday", 3, "week day", 4),
            ("tuesday", 1, "week day", 4),
            ("wednesday", 2, "week day", 4),
        ];
        for (idx, (day, day_num, day_type, refcount)) in cases.into_iter().enumerate() {
            assert_eq!(pointers[idx].0, day);
            assert_eq!(pointers[idx].1.meta, day_num);
            assert_eq!(
                storage[&pointers[idx].1.hash],
                MapValue {
                    value: day_type,
                    refcount,
                }
            );
        }

        // Remove the rest of the items
        map.remove("tuesday");
        map.remove("wednesday");
        map.remove("thursday");
        map.remove("friday");

        // Make sure the internal structure changed as expected
        assert_eq!(map.len_pointers(), 0);
        assert_eq!(map.len_storage(), 0);
        assert!(map.pointers.iter().next().is_none());
        assert!(map.storage.iter().next().is_none());
    }

    #[test]
    fn test_retain() {
        let map = DedupedMap::<&str, &str, &str>::new();
        map.insert("a", "red", "#f00");
        map.insert("b", "red", "#f00");
        map.insert("c", "green", "#0f0");
        map.insert("d", "green", "#0f0");
        map.insert("e", "blue", "#00f");
        map.insert("f", "blue", "#00f");

        assert_eq!(map.len_pointers(), 6);
        assert_eq!(map.len_storage(), 3);

        // retain only red things
        map.retain(|_key, meta, _value| ControlFlow::Continue(*meta == "red"));

        assert_eq!(map.len_pointers(), 2);
        assert_eq!(map.len_storage(), 1);

        assert!(map.contains_key(&"a"));
        assert!(map.contains_key(&"b"));
    }

    #[test]
    fn test_retain_control_flow() {
        let map = DedupedMap::<u32, (), ()>::new();
        for i in 0..10 {
            map.insert(i, (), ());
        }
        assert_eq!(map.len_pointers(), 10);
        assert_eq!(map.len_storage(), 1);

        // Remove numbers until we find five, and then stop.
        // note: keys are iterated in an arbitrary order.
        let mut removed_items = HashSet::new();
        map.retain(|key, _, _| {
            if *key == 5 {
                ControlFlow::Break
            } else {
                removed_items.insert(*key);
                ControlFlow::Continue(false)
            }
        });

        // Five should still be in the map, the removed numbers should no longer
        // be in the map, and any number that wasn't encountered before the
        // break should still be in the map.
        assert!(map.contains_key(&5));
        for i in 0..10 {
            if removed_items.contains(&i) {
                assert!(!map.contains_key(&i));
            } else {
                assert!(map.contains_key(&i));
            }
        }
    }

    #[test]
    fn test_remove_everything() {
        let map = DedupedMap::<char, (), usize>::new();
        for (idx, letter) in ('a'..='z').enumerate() {
            map.insert(letter, (), idx / 2);
        }

        assert_eq!(map.len_pointers(), 26);
        assert_eq!(map.len_storage(), 13);

        map.retain(|_, _, _| ControlFlow::Continue(false));

        assert_eq!(map.len_pointers(), 0);
        assert_eq!(map.len_storage(), 0);
    }
}
