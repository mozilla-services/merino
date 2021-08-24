//! Interactions with Redis.

mod domain;

use std::{borrow::Cow, convert::TryInto, time::Duration};

use crate::{domain::CacheKey, redis::domain::RedisSuggestions};
use anyhow::{anyhow, Context};
use async_trait::async_trait;
use merino_settings::Settings;
use merino_suggest::{
    CacheStatus, SetupError, SuggestError, Suggestion, SuggestionProvider, SuggestionRequest,
    SuggestionResponse,
};
use redis::RedisError;
use tracing_futures::{Instrument, WithSubscriber};
use uuid::Uuid;

use self::domain::RedisTtl;

/// A suggester that uses Redis to cache previous results.
pub struct Suggester<S> {
    /// The suggester to query on cache-miss.
    inner: S,

    /// Connection to Redis.
    redis_connection: redis::aio::ConnectionManager,

    /// The default amount of time a cache entry is valid, unless overridden by
    /// `inner`.
    default_ttl: Duration,

    /// Default lock timeout
    default_lock_timeout: Duration,
}

#[derive(Debug)]
/// The result of fetching an entry from the cache.
enum CacheCheckResult {
    /// The entry was found in the cache.
    Hit(SuggestionResponse),
    /// The entry was not found in the cache.
    Miss,
    /// There was an error retrieving the item from the cache that should be
    /// treated as a miss.
    ErrorAsMiss,
}

impl<S> Suggester<S>
where
    for<'a> S: SuggestionProvider<'a>,
{
    /// Create a Redis suggestion provider from settings that wraps `provider`.
    /// Opens a connection to Redis.
    ///
    /// # Errors
    /// Fails if it cannot connect to Redis.
    pub async fn new_boxed(settings: &Settings, provider: S) -> Result<Box<Self>, SetupError> {
        tracing::debug!(?settings.redis_cache.url, "Setting up redis connection");
        let client = redis::Client::open(settings.redis_cache.url.clone().ok_or_else(|| {
            SetupError::InvalidConfiguration(anyhow!("No Redis URL is configured for caching"))
        })?)
        .context("Setting up Redis client")
        .map_err(SetupError::Network)?;

        let redis_connection = redis::aio::ConnectionManager::new(client)
            .await
            .context("Connecting to Redis")
            .map_err(SetupError::Network)?;

        Ok(Box::new(Self {
            inner: provider,
            redis_connection,
            default_ttl: settings.redis_cache.default_ttl,
            default_lock_timeout: settings.redis_cache.default_lock_timeout,
        }))
    }

    /// Retrieve an item from the cache
    ///
    /// If the item retrieved cannot be deserialized, it will be deleted. If
    /// there is no TTL for the retrieved item, one will be added to it.
    async fn get_key(&self, key: &str) -> Result<CacheCheckResult, SuggestError> {
        let mut connection = self.redis_connection.clone();
        let span = tracing::info_span!("getting-cache-entry", %key);

        let cache_result: Result<(Option<RedisSuggestions>, RedisTtl), RedisError> = redis::pipe()
            .add_command(redis::Cmd::get(key))
            .add_command(redis::Cmd::ttl(key))
            .query_async(&mut connection)
            .instrument(span)
            .await;

        match cache_result {
            Ok((Some(suggestions), ttl)) => {
                let ttl = match ttl {
                    RedisTtl::KeyDoesNotExist => {
                        // This probably should never happen?
                        tracing::error!(%key, "Cache provided a suggestion but claims it doesn't exist for TTL determination");
                        self.default_ttl
                    }
                    RedisTtl::KeyHasNoTtl => {
                        tracing::warn!(%key, default_ttl = ?self.default_ttl, "Value in cache without TTL, setting default TTL");
                        self.queue_set_key_ttl(key, self.default_ttl)?;
                        self.default_ttl
                    }
                    RedisTtl::Ttl(t) => Duration::from_secs(t as u64),
                };
                Ok(CacheCheckResult::Hit(
                    SuggestionResponse::new(suggestions.0)
                        .with_cache_status(CacheStatus::Hit)
                        .with_cache_ttl(ttl),
                ))
            }

            Ok((None, _)) => Ok(CacheCheckResult::Miss),

            Err(error) => {
                match error.kind() {
                    redis::ErrorKind::TypeError => {
                        tracing::warn!(%error, %key, "Cached value not of expected type, deleting and treating as cache miss");
                        self.queue_delete_key(key)?;
                    }
                    _ => {
                        tracing::error!(%error, "Error reading suggestion from cache, treating as cache miss");
                    }
                }
                Ok(CacheCheckResult::ErrorAsMiss)
            }
        }
    }

    /// Generate a lock identifier key.
    /// This is a VERY simple locking mechanism. The only bit of fancy is that it will
    /// expire, allowing for "stuck" queries to eventually resolve.
    fn pending_key(key: &str) -> String {
        format!("pending_{}", key)
    }

    /// See iff a record update is pending. (Does not check lock value, only if it exists)
    async fn is_pending(&self, key: &str) -> Result<bool, SuggestError> {
        let pending_key = Self::pending_key(key);
        let mut connection = self.redis_connection.clone();

        tracing::trace!("🔒Checking key: {:?}", &pending_key);
        let lock = redis::pipe()
            .add_command(redis::Cmd::get(&pending_key))
            .query_async::<redis::aio::ConnectionManager, String>(&mut connection)
            .instrument(tracing::info_span!("getting-cache-pending", %pending_key))
            .await;

        let locked = !lock.unwrap_or_default().is_empty();
        tracing::trace!("🔒Is Pending {:?} {:?}", &pending_key, &locked);
        Ok(locked)
    }

    /// Check to see if the lock we have matches the lock we made.
    async fn check_pending(&self, key: &str, lock: &str) -> bool {
        let mut conn = self.redis_connection.clone();
        let pending_key = Self::pending_key(key);
        let locked = match redis::cmd("GET")
            .arg(&pending_key)
            .query_async::<redis::aio::ConnectionManager, String>(&mut conn)
            .await
        {
            Ok(set_lock) => {
                tracing::trace!(
                    "🔒Checking key: {:?} {:?} =? {:?}",
                    &pending_key,
                    set_lock,
                    lock
                );
                set_lock == lock
            }
            Err(e) => {
                tracing::warn!("🔒Could not get lock. Gone? {:?} {:?}", pending_key, e);
                false
            }
        };
        tracing::trace!("🔒Check Pending: {:?} {:?}", pending_key, locked);
        locked
    }

    /// Generate a lock, this returns a unique Lock value string to ensure that
    /// only the thread with the most recent "lock" can write to this key.
    async fn set_pending(&self, key: &str) -> Result<String, SuggestError> {
        let mut connection = self.redis_connection.clone();
        let pending_key = Self::pending_key(key);
        let lock = Uuid::new_v4().to_simple().to_string();
        tracing::trace!("🔒Setting lock for {:?} to {:?}", &pending_key, &lock);
        redis::pipe()
            .add_command(redis::Cmd::set(&pending_key, &lock))
            .add_command(redis::Cmd::expire(
                &pending_key,
                self.default_lock_timeout
                    .as_secs()
                    .try_into()
                    .unwrap_or(3),
            ))
            .query_async(&mut connection)
            .await
            .map_err(|e| {
                tracing::error!("🔒lock error: {:?}", e);
                SuggestError::Internal(e.into())
            })?;
        Ok(lock)
    }

    /// Delete a lock (fails if the lock value does not match.)
    async fn del_pending(&self, key: &str, lock: &str) -> Result<(), SuggestError> {
        let mut connection = self.redis_connection.clone();
        let pending_key = Self::pending_key(key);
        if self.check_pending(key, lock).await {
            tracing::trace!("🔒Removing lock for {:?}", &pending_key);
            redis::cmd("DEL")
                .arg(&pending_key)
                .query_async::<redis::aio::ConnectionManager, _>(&mut connection)
                .await
                .map_err(|e| {
                    tracing::warn!("🔒Key removal error: {:?} {:?}", &pending_key, e);
                    SuggestError::Internal(e.into())
                })?;
        } else {
            tracing::debug!("🔒Hrm, existing lock for {:?}", &pending_key);
        }
        Ok(())
    }

    /// Queue a command to store an entry in the cache.
    ///
    /// This runs as a separate task, and this function returns before the
    /// operation is complete.
    ///
    /// # Errors
    /// Returns an error if the command cannot be queued. Does *not* error if the
    /// command fails to run to completion.
    fn queue_store_key(&self, key: &str, suggestions: Vec<Suggestion>) -> Result<(), SuggestError> {
        let mut connection = self.redis_connection.clone();
        let key = key.to_string();
        let span = tracing::info_span!("storing-cache-entry", %key);
        let ttl = self.default_ttl.as_secs() as usize;

        tokio::task::spawn(async move {
            let to_store = RedisSuggestions(suggestions);
            tracing::debug!(%key, "storing cache entry");
            match redis::pipe()
                .add_command(redis::Cmd::set(&key, to_store))
                .add_command(redis::Cmd::expire(&key, ttl))
                .query_async(&mut connection)
                .await
            {
                Ok(()) => {
                    tracing::debug!(%key, "Successfully stored cache entry");
                }
                Err(error) => {
                    tracing::error!(%error, r#type="cache.redis.save-error", "Could not save suggestion to redis")
                }
            }

        }.with_current_subscriber().instrument(span));

        Ok(())
    }

    /// Queue a command to delete a key from the cache.
    ///
    /// This runs as a separate task, and this function returns before the
    /// deletion is complete.
    ///
    /// # Errors
    /// Returns an error if the command cannot be queued. Does *not* error if the
    /// command fails to run to completion.
    fn queue_delete_key(&self, key: &str) -> Result<(), SuggestError> {
        let mut connection = self.redis_connection.clone();
        let key = key.to_string();
        let span = tracing::info_span!("deleting-cache-entry", %key);

        tokio::task::spawn(
            async move {
                match redis::Cmd::del(&key).query_async(&mut connection).await {
                    Ok(()) => tracing::trace!("Successfully deleted cache key"),
                    Err(error) => tracing::error!(%error, "Couldn't delete cache key"),
                };
            }
            .with_current_subscriber()
            .instrument(span),
        );

        Ok(())
    }

    /// Queue a command to set the TTL of a key in the cache.
    ///
    /// This runs as a separate task, and this function returns before the
    /// operation is complete.
    ///
    /// # Errors
    /// Returns an error if the command cannot be queued. Does *not* error if the
    /// command fails to run to completion.
    fn queue_set_key_ttl(&self, key: &str, ttl: Duration) -> Result<(), SuggestError> {
        let mut connection = self.redis_connection.clone();
        let key = key.to_string();
        let span = tracing::info_span!("setting-cache-ttl", %key);

        tokio::task::spawn(
            async move {
                match redis::Cmd::expire(&key, ttl.as_secs() as usize)
                    .query_async(&mut connection)
                    .await
                {
                    Ok(()) => tracing::trace!("Successfully set TTL for cache key"),
                    Err(error) => tracing::error!(%error, "Couldn't delete cache key"),
                };
            }
            .with_current_subscriber()
            .instrument(span),
        );
        Ok(())
    }
}

#[async_trait]
impl<'a, S> SuggestionProvider<'a> for Suggester<S>
where
    S: for<'b> SuggestionProvider<'b> + Send + Sync,
{
    fn name(&self) -> Cow<'a, str> {
        format!("RedisCache({})", self.inner.name()).into()
    }

    async fn suggest(
        &self,
        request: SuggestionRequest<'a>,
    ) -> Result<SuggestionResponse, SuggestError> {
        let key = request.cache_key();

        let cache_result = self.get_key(&key).await?;

        if let CacheCheckResult::Hit(suggestions) = cache_result {
            tracing::debug!(%key, "cache hit");
            Ok(suggestions)
        } else {
            if self.is_pending(&key).await? {
                tracing::debug!(%key, "cache updating...");
                // A "pending" review may not yet have content (e.g. it's the initial lookup), otherwise it's a "Hit"
                let response =
                    SuggestionResponse::new(Vec::new()).with_cache_status(CacheStatus::Miss);
                return Ok(response);
            }
            let lock = self.set_pending(&key).await?;
            let mut response = self
                .inner
                .suggest(request)
                .await?
                .with_cache_ttl(self.default_ttl);

            if self.check_pending(&key, &lock).await {
                self.queue_store_key(&key, response.suggestions.clone())?;
                tracing::trace!("🔒Killing lock: {:?} {:?}", &key, &lock);
                self.del_pending(&key, &lock).await?;
            }

            if let CacheCheckResult::Miss = cache_result {
                tracing::debug!(%key, "cache miss");
                response = response.with_cache_status(CacheStatus::Miss);
            } else {
                debug_assert!(matches!(cache_result, CacheCheckResult::ErrorAsMiss));
                tracing::debug!(%key, "cache error treated as miss");
                response = response.with_cache_status(CacheStatus::Error);
            }
            Ok(response)
        }
    }
}
