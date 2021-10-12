#![warn(missing_docs)]

//! A data structure that can efficiently store many keys that map to a
//! relatively small number of values.

use dashmap::{mapref::entry::Entry, DashMap};
use std::collections::hash_map::DefaultHasher;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;

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

/// The second layer of the map, a reference counted value.
#[derive(Debug)]
struct MapValue<V> {
    /// The stored value.
    value: V,
    /// The number of pointers that are referring to this storage item.
    refcount: usize,
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
        if let Entry::Occupied(mut occupied_pointer_entry) = self.pointers.entry(key) {
            let pointer = occupied_pointer_entry.get_mut();
            match self.storage.entry(pointer.hash) {
                Entry::Occupied(mut occupied_storage_entry) => {
                    let item = occupied_storage_entry.get_mut();
                    if item.refcount > 1 {
                        item.refcount -= 1;
                    } else {
                        occupied_pointer_entry.remove();
                    }
                }
                Entry::Vacant(_) => {
                    tracing::error!(key = %key_desc, "Dangling storage entry");
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
                    tracing::error!(?key, "Dangling storage entry");
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
                    tracing::error!("missing storage entry in memory cache");
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
    use std::collections::HashSet;

    use super::{ControlFlow, DedupedMap};

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
}
