//! A cache system that uses local in-memory storage to store a limited number of items.
//!
//! The cache system here is actually two tiered. The first tier maps from
//! suggestion requests to the hash of the response they should use. The second
//! tier maps from those hashes to the responses. In this way, duplicate
//! responses can be stored only once, even if they are used for many requests.

use crate::{
    deduped_map::{ControlFlow, DedupedMap},
    domain::CacheKey,
};
use async_trait::async_trait;
use lazy_static::lazy_static;
use merino_settings::Settings;
use merino_suggest::{
    CacheStatus, Suggestion, SuggestionProvider, SuggestionRequest, SuggestionResponse,
};
use std::{
    collections::HashMap,
    fmt::Debug,
    hash::Hash,
    sync::Arc,
    time::{Duration, Instant},
};
use tracing::Instrument;

use arc_swap::ArcSwap;

lazy_static! {
    static ref LOCK_TABLE: ArcSwap<HashMap<String, Instant>> =
        ArcSwap::from_pointee(HashMap::new());
}

impl LOCK_TABLE {
    /// Check to see if there's any lock for a given key
    fn is_locked(&self, key: &str) -> bool {
        if let Some(lock_val) = self.load().get(key) {
            return *lock_val > Instant::now();
        }
        false
    }

    /// Generate a lock for the given key and timeout
    fn add_lock(&self, key: &str, lock_timeout: Duration) -> Instant {
        let lock = Instant::now() + lock_timeout;
        self.rcu(|table| {
            let mut locked = HashMap::clone(table);
            locked.insert(key.to_owned(), lock);
            locked
        });
        lock
    }

    /// run func and remove lock, only if the lock we have matches what
    /// is registered for the key.
    fn update<F>(&self, key: &str, lock: Instant, mut func: F)
    where
        F: FnMut(),
    {
        self.rcu(|table| {
            let mut locked = HashMap::clone(table);
            if let Some(ts) = locked.get(key) {
                if *ts == lock {
                    func();
                }
                locked.remove(key);
            }
            locked
        });
    }

    /// remove any expired elements from the Pending table
    /// (There shouldn't be many.)
    fn prune(&self, start: &Instant) {
        self.rcu(|table| {
            let mut cleaned = HashMap::clone(table);
            cleaned.retain(|_k, v| *v > *start);
            cleaned
        });
    }
}

/// A in-memory cache for suggestions.
pub struct Suggester {
    /// The suggester to query on cache-miss.
    inner: Box<dyn SuggestionProvider>,

    /// The cached items.
    items: Arc<DedupedMap<String, Instant, Vec<Suggestion>>>,

    /// TTL to apply to items if the underlying provider does not give one.
    default_ttl: Duration,

    /// TTL for locks on cache refresh updates
    default_lock_timeout: Duration,
}

impl Suggester {
    /// Create a in-memory suggestion cache from settings that wraps `provider`.
    pub fn new_boxed(settings: &Settings, provider: Box<dyn SuggestionProvider>) -> Box<Self> {
        let items = Arc::new(DedupedMap::new());

        {
            let task_items = items.clone();
            let task_interval = settings.memory_cache.cleanup_interval;
            tokio::spawn(async move {
                let mut timer = tokio::time::interval(task_interval);
                // The timer fires immediately, but we don't want to run the
                // cleanup function immediately, so wait one tick before
                // starting the loop.
                timer.tick().await;
                loop {
                    timer.tick().await;
                    Self::remove_expired_entries(&task_items);
                }
            });
        }

        Box::new(Self {
            inner: provider,
            items,
            default_ttl: settings.memory_cache.default_ttl,
            default_lock_timeout: settings.memory_cache.default_lock_timeout,
        })
    }

    /// Remove expired entries from `items`
    ///
    /// This is a selfless method so that it can be called from a spawned Tokio task.
    #[tracing::instrument(level = "debug", skip(items))]
    fn remove_expired_entries<K: Eq + Hash + Debug>(
        items: &Arc<DedupedMap<K, Instant, Vec<Suggestion>>>,
    ) {
        let start = Instant::now();
        let count_before_storage = items.len_storage();
        let count_before_pointers = items.len_pointers();

        // Retain all cache entries that have not yet expired.
        let max_removals = 10_000;
        let mut num_removals = 0;
        items.retain(|_key, expiration, _suggestions| {
            if num_removals > max_removals {
                tracing::warn!(
                    ?max_removals,
                    "memory-cache cleanup reached max number of removed entries"
                );
                return ControlFlow::Break;
            }

            let should_remove = *expiration < start;
            if should_remove {
                num_removals += 1;
            }
            ControlFlow::Continue(!should_remove)
        });

        LOCK_TABLE.prune(&start);

        // Report finishing.
        let duration = Instant::now() - start;
        let removed_storage = count_before_storage - items.len_storage();
        let removed_pointers = count_before_pointers - items.len_pointers();
        tracing::info!(
            ?duration,
            ?removed_pointers,
            ?removed_storage,
            "finished removing expired entries from cache"
        );
    }
}

#[async_trait]
impl SuggestionProvider for Suggester {
    fn name(&self) -> String {
        "in-memory-cache".into()
    }

    async fn suggest(
        &self,
        query: SuggestionRequest,
    ) -> Result<SuggestionResponse, merino_suggest::SuggestError> {
        let now = Instant::now();
        let key = query.cache_key().to_string();
        let span = tracing::debug_span!("memory-suggest", ?key);
        async move {
            tracing::debug!("suggesting with memory cache");

            match self.items.get(&key) {
                Some((expiration, _)) if expiration <= now => {
                    tracing::debug!("cache expired");
                    self.items.remove(key.clone());
                }
                Some((expiration, suggestions)) => {
                    tracing::debug!("cache hit");
                    return Ok(SuggestionResponse {
                        cache_status: CacheStatus::Hit,
                        cache_ttl: Some(expiration - now),
                        suggestions,
                    });
                }
                None => {
                    tracing::debug!("cache miss");
                }
            }

            if LOCK_TABLE.is_locked(&key) {
                // there's a fetch already in progress. Return empty for now.
                return Ok(SuggestionResponse {
                    cache_status: CacheStatus::Hit,
                    cache_ttl: None,
                    suggestions: Vec::new(),
                });
            }

            // handle cache miss or stale cache
            let lock = LOCK_TABLE.add_lock(&key, self.default_lock_timeout);
            let mut response = self
                .inner
                .suggest(query)
                .await?
                // Todo, cache status should be a vec.
                .with_cache_status(CacheStatus::Miss);

            LOCK_TABLE.update(&key, lock, || {
                // Update the cache data.
                let cache_ttl = response.cache_ttl.get_or_insert(self.default_ttl);
                let expiration = now + *cache_ttl;
                tracing::debug!(?now, ?expiration, "inserting into cache");
                self.items
                    .insert(key.clone(), expiration, response.suggestions.clone());
            });
            Ok(response)
        }
        .instrument(span)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::{Suggester, LOCK_TABLE};
    use crate::deduped_map::DedupedMap;
    use fake::{Fake, Faker};
    use merino_suggest::Suggestion;
    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };

    #[test]
    fn cache_maintainer_removes_expired_entries() {
        let cache: Arc<DedupedMap<String, Instant, Vec<Suggestion>>> = Arc::new(DedupedMap::new());

        let suggestions = vec![Faker.fake()];
        cache.insert(
            "expired".to_string(),
            Instant::now() - Duration::from_secs(300),
            suggestions.clone(),
        );
        cache.insert(
            "current".to_string(),
            Instant::now() + Duration::from_secs(300),
            suggestions,
        );
        assert_eq!(cache.len_storage(), 1);
        assert_eq!(cache.len_pointers(), 2);
        assert!(cache.contains_key(&"current".to_owned()));
        assert!(cache.contains_key(&"expired".to_owned()));

        Suggester::remove_expired_entries(&cache);

        assert_eq!(cache.len_storage(), 1);
        assert_eq!(cache.len_pointers(), 1);
        assert!(cache.contains_key(&"current".to_owned()));
        assert!(!cache.contains_key(&"expired".to_owned()));
    }

    #[test]
    fn cache_lock_test() {
        let lock_name = "testLock";
        let other_lock_name = "otherLock";
        let timeout = Duration::from_secs(3);
        let lock = LOCK_TABLE.add_lock(lock_name, timeout);
        let mut lock_check = false;
        LOCK_TABLE.add_lock(other_lock_name, timeout);
        assert!(LOCK_TABLE.is_locked(lock_name));
        assert!(!LOCK_TABLE.is_locked("unlocked"));

        LOCK_TABLE.update(lock_name, lock, || lock_check = true);

        assert!(lock_check);
        assert!(!LOCK_TABLE.is_locked(lock_name));

        // Should fail, lock dismissed
        LOCK_TABLE.update(lock_name, lock, || lock_check = false);
        assert!(lock_check);

        // Should fail, wrong lock value
        LOCK_TABLE.update(other_lock_name, lock, || lock_check = false);
        assert!(lock_check);
    }
}
