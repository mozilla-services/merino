//! A cache system that uses local in-memory storage to store a limited number of items.
//!
//! The cache system here is actually two tiered. The first tier maps from
//! suggestion requests to the hash of the response they should use. The second
//! tier maps from those hashes to the responses. In this way, duplicate
//! responses can be stored only once, even if they are used for many requests.

use anyhow::anyhow;
use async_trait::async_trait;
use cadence::{CountedExt, Gauged, StatsdClient};
use deduped_dashmap::{ControlFlow, DedupedMap};
use lazy_static::lazy_static;
use merino_settings::providers::MemoryCacheConfig;
use merino_suggest::{
    metrics::TimedMicros, CacheInputs, CacheStatus, Suggestion, SuggestionProvider,
    SuggestionRequest, SuggestionResponse,
};
use std::{
    collections::HashMap,
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
#[derive(Clone)]
pub struct Suggester {
    /// The suggester to query on cache-miss.
    inner: Arc<Box<dyn SuggestionProvider>>,

    /// The Statsd client used to record statistics.
    metrics_client: StatsdClient,

    /// The cached items.
    items: Arc<DedupedMap<String, Instant, Vec<Suggestion>>>,

    /// TTL to apply to items if the underlying provider does not give one.
    default_ttl: Duration,

    /// TTL for locks on cache refresh updates
    default_lock_timeout: Duration,

    /// Maximum number of entries to remove in a single background expiration iteration.
    max_background_removals: usize,
}

impl Suggester {
    /// Create a in-memory suggestion cache from settings that wraps `provider`.
    #[must_use]
    pub fn new_boxed(
        config: &MemoryCacheConfig,
        provider: Box<dyn SuggestionProvider>,
        metrics_client: StatsdClient,
    ) -> Box<Self> {
        let suggester = Self {
            inner: Arc::new(provider),
            metrics_client,
            items: Arc::new(DedupedMap::new()),
            default_ttl: config.default_ttl,
            default_lock_timeout: config.default_lock_timeout,
            max_background_removals: config.max_removed_entries,
        };

        {
            let cloned_suggester = suggester.clone();
            let task_interval = config.cleanup_interval;
            tokio::spawn(async move {
                let mut timer = tokio::time::interval(task_interval);
                // The timer fires immediately, but we don't want to run the
                // cleanup function immediately, so wait one tick before
                // starting the loop.
                timer.tick().await;
                loop {
                    timer.tick().await;
                    let mut suggester = cloned_suggester.clone();

                    // Dispatch the expiry task to the blocking threads of the
                    // runtime. This prevents the expiry task, which is inherently
                    // blocking, from blocking the other tasks running on the
                    // core threads of the runtime.
                    tokio::task::spawn_blocking(move || {
                        suggester.remove_expired_entries();
                    });
                }
            });
        }

        Box::new(suggester)
    }

    /// Remove expired entries from `items`
    #[tracing::instrument(level = "debug", skip(self))]
    fn remove_expired_entries(&mut self) {
        let start = Instant::now();
        let count_before_storage = self.items.len_storage();
        let count_before_pointers = self.items.len_pointers();

        // Retain all cache entries that have not yet expired.
        let mut num_removals = 0;
        self.items.retain(|_key, expiration, _suggestions| {
            if num_removals > self.max_background_removals {
                tracing::warn!(
                    r#type = "cache.memory.max-removals",
                    ?self.max_background_removals,
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
        let removed_storage = count_before_storage - self.items.len_storage();
        let removed_pointers = count_before_pointers - self.items.len_pointers();
        tracing::info!(
            r#type = "cache.memory.remove-expired",
            ?duration,
            ?removed_pointers,
            ?removed_storage,
            "finished removing expired entries from cache"
        );

        self.metrics_client
            .gauge("cache.memory.storage-len", self.items.len_storage() as u64)
            .ok();
        self.metrics_client
            .gauge(
                "cache.memory.pointers-len",
                self.items.len_pointers() as u64,
            )
            .ok();
    }
}

#[async_trait]
impl SuggestionProvider for Suggester {
    fn name(&self) -> String {
        format!("MemoryCache({})", self.inner.name())
    }

    fn cache_inputs(&self, req: &SuggestionRequest, cache_inputs: &mut dyn CacheInputs) {
        self.inner.cache_inputs(req, cache_inputs);
    }

    async fn suggest(
        &self,
        query: SuggestionRequest,
    ) -> Result<SuggestionResponse, merino_suggest::SuggestError> {
        let now = Instant::now();
        let key = self.cache_key(&query);
        let span = tracing::debug_span!("memory-suggest", ?key);

        // closure for `span`.
        async move {
            tracing::debug!("suggesting with memory cache");
            let mut rv = None;

            match self.items.get(&key) {
                Some((expiration, _)) if expiration <= now => {
                    tracing::debug!("cache expired");
                    self.items.remove(key.clone());
                }
                Some((expiration, suggestions)) => {
                    tracing::debug!("cache hit");
                    self.metrics_client.incr("cache.memory.hit").ok();
                    rv = Some(SuggestionResponse {
                        cache_status: CacheStatus::Hit,
                        cache_ttl: Some(expiration - now),
                        suggestions,
                    });
                }
                None => {
                    tracing::debug!("cache miss");
                    self.metrics_client.incr("cache.memory.miss").ok();
                }
            }

            if rv.is_none() {
                if LOCK_TABLE.is_locked(&key) {
                    // There's a fetch already in progress. Return empty for now.
                    rv = Some(SuggestionResponse {
                        cache_status: CacheStatus::Hit,
                        cache_ttl: None,
                        suggestions: Vec::new(),
                    });
                } else {
                    // Handle cache miss or stale cache.
                    let lock = LOCK_TABLE.add_lock(&key, self.default_lock_timeout);
                    let mut response = self
                        .inner
                        .suggest(query)
                        .await?
                        .with_cache_status(CacheStatus::Miss);

                    LOCK_TABLE.update(&key, lock, || {
                        // Update the cache data.
                        let cache_ttl = response.cache_ttl.get_or_insert(self.default_ttl);
                        let expiration = now + *cache_ttl;
                        tracing::debug!(?now, ?expiration, "inserting into cache");
                        self.items
                            .insert(key.clone(), expiration, response.suggestions.clone());
                    });

                    rv = Some(response);
                }
            }

            if let Some(response) = rv {
                self.metrics_client
                    .time_micros_with_tags("cache.memory.duration-us", now.elapsed())
                    .with_tag("cache-status", response.cache_status.to_string().as_str())
                    .send();
                Ok(response)
            } else {
                Err(merino_suggest::SuggestError::Internal(anyhow!(
                    "No result generated"
                )))
            }
        }
        .instrument(span)
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::{Suggester, LOCK_TABLE};
    use cadence::{SpyMetricSink, StatsdClient};
    use deduped_dashmap::DedupedMap;
    use fake::{Fake, Faker};
    use merino_suggest::{NullProvider, Suggestion};
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

        // Provide an inspectable metrics sink to validate the collected data.
        let (rx, sink) = SpyMetricSink::new();
        let metrics_client = StatsdClient::from_sink("merino-test", sink);

        let mut suggester = Suggester {
            inner: Arc::new(Box::new(NullProvider)),
            metrics_client,
            items: cache.clone(),
            default_ttl: Duration::from_secs(30),
            default_lock_timeout: Duration::from_secs(1),
            max_background_removals: usize::MAX,
        };

        suggester.remove_expired_entries();

        assert_eq!(cache.len_storage(), 1);
        assert_eq!(cache.len_pointers(), 1);
        assert!(cache.contains_key(&"current".to_owned()));
        assert!(!cache.contains_key(&"expired".to_owned()));

        // Verify the reported metric.
        assert_eq!(rx.len(), 2);
        let collected_data: Vec<String> = rx
            .iter()
            .take(2)
            .map(|x| String::from_utf8(x).unwrap())
            .collect();
        dbg!(&collected_data);
        assert!(collected_data.contains(&"merino-test.cache.memory.storage-len:1|g".to_string()));
        assert!(collected_data.contains(&"merino-test.cache.memory.pointers-len:1|g".to_string()));
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
