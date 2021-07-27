//! A cache system that uses local in-memory storage to store a limited number of items.

use crate::domain::CacheKey;
use async_trait::async_trait;
use dashmap::{mapref::entry::Entry, DashMap};
use merino_settings::Settings;
use merino_suggest::{
    CacheStatus, Suggestion, SuggestionProvider, SuggestionRequest, SuggestionResponse,
};
use std::{
    borrow::Cow,
    time::{Duration, Instant},
};
use tracing::Instrument;

/// An entry in the in-memory store, that includes an expiration time.
#[derive(Debug)]
struct CacheEntry {
    /// The suggestions to provide with the cache. This differs from
    /// [`SuggestionResponse`] because we store expiration date as an explicit
    /// expiration date instead of a TTL, and convert to and from
    /// `SuggestionResponse`.
    suggestions: Vec<Suggestion>,

    /// After this time, the cache entry should no longer be considered valid,
    /// and should be removed.
    expiration: Instant,
}

/// A in-memory cache for suggestions.
pub struct Suggester<S> {
    /// The suggester to query on cache-miss.
    inner: S,

    /// The items stored in the cache.
    items: DashMap<String, CacheEntry>,

    /// TTL to apply to items if the underlying provider does not give one.
    default_ttl: Duration,
}

impl<S> Suggester<S> {
    /// Create a in-memory suggestion cache from settings that wraps `provider`.
    pub fn new_boxed(settings: &Settings, provider: S) -> Box<Self> {
        Box::new(Self {
            inner: provider,
            items: DashMap::new(),
            default_ttl: settings.memory_cache.default_ttl,
        })
    }
}

#[async_trait]
impl<'a, S> SuggestionProvider<'a> for Suggester<S>
where
    S: for<'b> SuggestionProvider<'b> + Send + Sync,
{
    fn name(&self) -> Cow<'a, str> {
        /// The name of the provider.
        static NAME: &str = "in-memory-cache";
        Cow::from(NAME)
    }

    async fn suggest(
        &self,
        query: SuggestionRequest<'a>,
    ) -> Result<SuggestionResponse, merino_suggest::SuggestError> {
        let now = Instant::now();
        let key = query.cache_key().to_string();
        let span = tracing::debug_span!("memory-suggest", ?key);
        async move {
            tracing::debug!("suggesting with memory cache");

            // Put the cache-check in a block so that `entry` drops before we
            // try and write back to the cache. If `entry` is not dropped by the
            // time we write cache misses into `self.items`, we may deadlock!
            {
                let entry = self.items.entry(key.clone());

                if let Entry::Occupied(occupied_entry) = entry {
                    let cache_entry = occupied_entry.get();
                    if now >= cache_entry.expiration {
                        tracing::debug!("cache expired");
                        occupied_entry.remove();
                    } else {
                        tracing::debug!("cache hit");
                        return Ok(SuggestionResponse {
                            cache_status: merino_suggest::CacheStatus::Hit,
                            cache_ttl: Some(cache_entry.expiration - now),
                            suggestions: cache_entry.suggestions.clone(),
                        });
                    }
                } else {
                    tracing::debug!("cache miss");
                }
            }

            // handle cache miss
            let mut response = self
                .inner
                .suggest(query)
                .await?
                // Todo, cache status should be a vec.
                .with_cache_status(CacheStatus::Miss);

            if response.cache_ttl.is_none() {
                response = response.with_cache_ttl(self.default_ttl);
            }

            let expiration = now + response.cache_ttl.unwrap_or(self.default_ttl);
            tracing::debug!(?now, ?expiration, "inserting into cache");
            self.items.insert(
                key,
                CacheEntry {
                    suggestions: response.suggestions.clone(),
                    expiration,
                },
            );

            Ok(response)
        }
        .instrument(span)
        .await
    }
}
