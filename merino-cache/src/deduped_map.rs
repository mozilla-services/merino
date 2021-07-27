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
#[derive(Debug, Default)]
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
    K: Eq + Hash,
    M: Debug + Clone,
    V: Hash + Debug + Clone,
{
    /// Create an empty map.
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

        self.pointers.insert(key, MapPointer { meta, hash });

        match self.storage.entry(hash) {
            Entry::Occupied(mut occupied_entry) => {
                occupied_entry.get_mut().refcount += 1;
            }
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(MapValue { value, refcount: 1 });
            }
        }
    }

    /// Remove the item associated with a key from the map.
    ///
    /// The metadata associated with `key` will always be removed. The reference
    /// count of the storage item it points to will be decremented. If no more
    /// keys refer to the storage item, it will also be removed.
    pub fn remove(&self, key: K) {
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
                    tracing::error!("missing storage entry in memory cache")
                }
            }
        }
    }

    /// Get cloned copies of the metadata and value associated with `key`.
    pub fn get(&self, key: &K) -> Option<(M, V)> {
        match self.pointers.get(key) {
            Some(pointer_ref) => match self.storage.get(&pointer_ref.hash) {
                Some(storage_ref) => {
                    let meta = pointer_ref.meta.clone();
                    let value = storage_ref.value.clone();
                    Some((meta, value))
                }
                None => {
                    tracing::error!("missing storage entry in memory cache");
                    self.pointers.remove(key);
                    None
                }
            },
            None => None,
        }
    }

    /// Fetches the total number of storage items in the map.
    ///
    /// This will be at most `self.len_pointers()`.
    pub fn len_storage(&self) -> usize {
        self.storage.len()
    }

    /// Fetches the total number of pointers stored in the map.
    ///
    /// This will be at least `self.len_storage()`.
    pub fn len_pointers(&self) -> usize {
        self.pointers.len()
    }

    /// Retain elements that whose predicates return true
    /// and discard elements whose predicates return false.
    pub fn retain<F>(&self, pred: F)
    where
        F: Fn(&K, &M, &V) -> bool,
    {
        self.pointers.retain(|key, pointer| -> bool {
            match self.storage.entry(pointer.hash) {
                Entry::Occupied(mut occupied_entry) => {
                    let item = occupied_entry.get_mut();
                    let should_keep = pred(key, &pointer.meta, &item.value);

                    if !should_keep {
                        if item.refcount > 1 {
                            item.refcount -= 1;
                        } else {
                            occupied_entry.remove();
                        }
                    }

                    should_keep
                }
                Entry::Vacant(_) => {
                    tracing::error!("missing storage entry in memory cache");
                    false
                }
            }
        })
    }

    /// Checks if the map contains a specific key.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn contains_key(&self, key: &K) -> bool {
        self.pointers.contains_key(key)
    }
}
