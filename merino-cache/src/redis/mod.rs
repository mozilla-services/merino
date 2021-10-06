//! Interactions with Redis.

mod domain;

use std::{convert::TryInto, time::Duration};

use crate::{domain::CacheKey, redis::domain::RedisSuggestions};
use anyhow::Context;
use async_trait::async_trait;
use fix_hidden_lifetime_bug::fix_hidden_lifetime_bug;
use merino_settings::{providers::RedisCacheConfig, Settings};
use merino_suggest::{
    CacheStatus, SetupError, SuggestError, Suggestion, SuggestionProvider, SuggestionRequest,
    SuggestionResponse,
};
use redis::RedisError;
use tracing_futures::{Instrument, WithSubscriber};
use uuid::Uuid;

use self::domain::RedisTtl;

/// A suggester that uses Redis to cache previous results.
pub struct Suggester {
    /// The suggester to query on cache-miss.
    inner: Box<dyn SuggestionProvider>,

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

#[derive(Clone)]
/// Very simple Redis Lock mechanism.
pub struct SimpleRedisLock {
    /// connection handler
    connection: redis::aio::ConnectionManager,
}

impl From<&redis::aio::ConnectionManager> for SimpleRedisLock {
    fn from(connection: &redis::aio::ConnectionManager) -> Self {
        Self {
            connection: connection.clone(),
        }
    }
}

impl SimpleRedisLock {
    /// Generate a lock identifier key.
    ///
    /// This is a VERY simple locking mechanism. The only bit of fancy is that it will
    /// expire, allowing for "stuck" queries to eventually resolve.
    fn lock_key(key: &str) -> String {
        format!("pending_{}", key)
    }

    /// See if a record update is locked for pending update.
    ///
    /// This does not check lock value, only if a lock exists.
    async fn is_locked(&mut self, key: &str) -> Result<bool, SuggestError> {
        let lock_key = Self::lock_key(key);

        tracing::trace!(%lock_key, "🔒Checking key");
        let lock = redis::Cmd::get(&lock_key)
            .query_async::<redis::aio::ConnectionManager, String>(&mut self.connection)
            .instrument(tracing::info_span!("getting-cache-pending", %lock_key))
            .await;

        let locked = !lock.unwrap_or_default().is_empty();
        tracing::trace!(%lock_key, "🔒Is Pending with {:?}", &locked);
        Ok(locked)
    }

    /// Generate a lock, this returns a unique Lock value string to ensure that
    /// only the thread with the most recent "lock" can write to this key.
    ///
    /// This will return a None if the lock could not be created.
    async fn lock(
        &mut self,
        key: &str,
        default_lock_timeout: Duration,
    ) -> Result<Option<String>, SuggestError> {
        let lock_key = Self::lock_key(key);
        let lock = Uuid::new_v4().to_simple().to_string();
        tracing::trace!("🔒Setting lock for {:?} to {:?}", &lock_key, &lock);
        if let Some(v) = redis::Cmd::set(&lock_key, &lock)
            .arg("NX")
            .arg("EX")
            .arg(default_lock_timeout.as_secs().try_into().unwrap_or(3))
            .query_async::<redis::aio::ConnectionManager, Option<String>>(&mut self.connection)
            .await
            .map_err(|e| {
                tracing::error!("🔒⛔lock error: {:?}", e);
                SuggestError::Internal(e.into())
            })?
        {
            if v == *"OK" {
                return Ok(Some(lock));
            }
        };
        Ok(None)
    }

    /// Only write a given item if the lock matches the value we have on hand.
    ///
    /// Silently fails and discards if the lock is invalid.
    async fn write_if_locked(
        &mut self,
        key: &str,
        lock: &str,
        to_store: RedisSuggestions,
        ttl: Duration,
    ) -> Result<(), SuggestError> {
        let lock_key = Self::lock_key(key);
        tracing::debug!(%key, "🔒 attempting to store cache entry");
        // atomically check the lock to make sure it matches our stored
        // value, and if it does write the corresponding storage and
        // delete the lock.
        let cmd = r"
            if redis.call('get', ARGV[1]) == ARGV[2] then
                redis.call('set', ARGV[3], ARGV[4], 'EX', tonumber(ARGV[5]))
                redis.call('del', ARGV[1])
                return true
            else
                return false
            end";
        tracing::trace!(%cmd, %lock_key, %lock, %key, "{}", ttl.as_secs());
        match redis::cmd("EVAL")
            .arg(cmd)
            .arg(0) // You need to specify the keys, even if you don't have any.
            .arg(lock_key) // argv[1]
            .arg(lock) // argv[2]
            .arg(key) // argv[3]
            .arg(to_store) // argv[4]
            .arg(ttl.as_secs()) // argv[5]
            .query_async::<redis::aio::ConnectionManager, bool>(&mut self.connection)
            .await
        {
            Ok(v) => {
                if v {
                    tracing::debug!(%key, "🔒Successfully stored cache entry");
                } else {
                    tracing::warn!(%key, "🔒⛔write blocked, newer lock");
                }
                Ok(())
            }
            Err(error) => {
                tracing::error!(%error, r#type="cache.redis.save-error", "Could not save suggestion to redis");
                Err(SuggestError::Internal(error.into()))
            }
        }
    }
}

impl Suggester {
    /// Create a Redis suggestion provider from settings that wraps `provider`.
    /// Opens a connection to Redis.
    ///
    /// # Errors
    /// Fails if it cannot connect to Redis.

    #[allow(clippy::manual_async_fn)]
    #[fix_hidden_lifetime_bug]
    pub async fn new_boxed(
        settings: &Settings,
        config: &RedisCacheConfig,
        provider: Box<dyn SuggestionProvider + 'static>,
    ) -> Result<Box<Self>, SetupError> {
        tracing::debug!(?settings.redis.url, "Setting up redis connection");
        let client = redis::Client::open(settings.redis.url.clone())
            .context("Setting up Redis client")
            .map_err(SetupError::Network)?;

        let redis_connection = redis::aio::ConnectionManager::new(client)
            .await
            .context(format!(
                "Connecting to Redis at {}",
                settings.redis.redacted_url(),
            ))
            .map_err(SetupError::Network)?;

        Ok(Box::new(Suggester {
            inner: provider,
            redis_connection,
            default_ttl: config.default_ttl,
            default_lock_timeout: config.default_lock_timeout,
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

    /// Queue a command to store an entry in the cache.
    ///
    /// This runs as a separate task, and this function returns before the
    /// operation is complete.
    ///
    /// # Errors
    /// Returns an error if the command cannot be queued. Does *not* error if the
    /// command fails to run to completion.
    fn queue_store_key(
        &self,
        key: &str,
        suggestions: Vec<Suggestion>,
        lock: String,
    ) -> Result<(), SuggestError> {
        let connection = self.redis_connection.clone();
        let key = key.to_string();
        let span = tracing::info_span!("storing-cache-entry", %key);
        let ttl = self.default_ttl;

        tokio::task::spawn(
            async move {
                let mut rlock = SimpleRedisLock::from(&connection);
                let to_store = RedisSuggestions(suggestions);
                rlock
                    .write_if_locked(&key, &lock, to_store, ttl)
                    .await
                    .expect("Could not write data");
            }
            .with_current_subscriber()
            .instrument(span),
        );
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
impl SuggestionProvider for Suggester {
    fn name(&self) -> String {
        format!("RedisCache({})", self.inner.name())
    }

    async fn suggest(
        &self,
        request: SuggestionRequest,
    ) -> Result<SuggestionResponse, SuggestError> {
        let key = request.cache_key();
        let mut rlock = SimpleRedisLock::from(&self.redis_connection);

        let cache_result = self.get_key(&key).await?;

        if let CacheCheckResult::Hit(suggestions) = cache_result {
            tracing::debug!(%key, "cache hit");
            Ok(suggestions)
        } else {
            if rlock.is_locked(&key).await? {
                tracing::debug!(%key, "cache updating...");
                // A "pending" review may not yet have content (e.g. it's the initial lookup), otherwise it's a "Hit"
                let response =
                    SuggestionResponse::new(Vec::new()).with_cache_status(CacheStatus::Miss);
                return Ok(response);
            }
            let response = if let Some(lock) = rlock.lock(&key, self.default_lock_timeout).await? {
                let response = self
                    .inner
                    .suggest(request)
                    .await?
                    .with_cache_ttl(self.default_ttl);

                self.queue_store_key(&key, response.suggestions.clone(), lock)?;

                if let CacheCheckResult::Miss = cache_result {
                    tracing::debug!(%key, "cache miss");
                    response.with_cache_status(CacheStatus::Miss)
                } else {
                    debug_assert!(matches!(cache_result, CacheCheckResult::ErrorAsMiss));
                    tracing::debug!(%key, "cache error treated as miss");
                    response.with_cache_status(CacheStatus::Error)
                }
            } else {
                SuggestionResponse::new(Vec::new()).with_cache_status(CacheStatus::Error)
            };
            Ok(response)
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use crate::redis::{domain::RedisSuggestions, SimpleRedisLock};

    use super::SetupError;
    use anyhow::Context;
    use http::Uri;
    use merino_settings::Settings;
    use merino_suggest::{Proportion, Suggestion};

    #[tokio::test]
    async fn check_cache() -> Result<(), SetupError> {
        let settings = Settings::load_for_tests();

        let r_client = redis::Client::open(settings.redis.url.clone())
            .context("Setting up Redis client")
            .map_err(SetupError::Network)?;
        let mut redis_connection = redis::aio::ConnectionManager::new(r_client)
            .await
            .context("Connecting to Redis")
            .map_err(SetupError::Network)?;

        // try to add an entry:
        let tty = Duration::from_secs(300);
        let mut rlock = SimpleRedisLock::from(&redis_connection);
        let test_key = "testKey";
        let uri: Uri = "https://example.com".parse().unwrap();
        let text = "test".to_owned();
        let suggestion1 = Suggestion {
            id: 1234,
            full_keyword: text.clone(),
            title: text.clone(),
            url: uri.clone(),
            impression_url: uri.clone(),
            click_url: uri.clone(),
            provider: text.clone(),
            is_sponsored: false,
            icon: uri.clone(),
            score: Proportion::zero(),
        };
        let suggestion2 = Suggestion {
            id: 5678,
            ..suggestion1.clone()
        };
        let to_store: Vec<Suggestion> = [suggestion1].to_vec();
        let to_store2: Vec<Suggestion> = [suggestion2].to_vec();

        // try a happy path write cycle.
        let lock = rlock
            .lock(test_key, Duration::from_secs(3))
            .await
            .expect("Could not generate lock")
            .unwrap();
        assert!(rlock.is_locked(test_key).await.expect("failed lock check"));
        assert!(rlock
            .write_if_locked(test_key, &lock, RedisSuggestions(to_store.clone()), tty)
            .await
            .is_ok());
        assert!(!rlock
            .is_locked(test_key)
            .await
            .expect("Could not check unlocked"));

        // test to see if you can re-lock.
        let lock2 = rlock.lock(test_key, tty).await.unwrap().unwrap();
        // second lock should fail.
        assert!(rlock.lock(test_key, tty).await.unwrap().is_none());

        let res1 = redis::Cmd::get(test_key)
            .query_async::<redis::aio::ConnectionManager, String>(&mut redis_connection)
            .await
            .unwrap();
        // trying to write with an old lock should silently fail.
        assert!(rlock
            .write_if_locked(test_key, &lock, RedisSuggestions(to_store2.clone()), tty)
            .await
            .is_ok());
        let res2 = redis::Cmd::get(test_key)
            .query_async::<redis::aio::ConnectionManager, String>(&mut redis_connection)
            .await
            .unwrap();
        assert_eq!(res1, res2, "cached values should match");

        // trying to write with an new lock should work and release the lock.
        assert!(rlock
            .write_if_locked(test_key, &lock2, RedisSuggestions(to_store2), tty)
            .await
            .is_ok());
        assert!(!rlock
            .is_locked(test_key)
            .await
            .expect("Could not check unlocked"));
        let res2 = redis::Cmd::get(test_key)
            .query_async::<redis::aio::ConnectionManager, String>(&mut redis_connection)
            .await
            .unwrap();
        assert_ne!(res1, res2, "cached values should not match");

        Ok(())
    }
}
