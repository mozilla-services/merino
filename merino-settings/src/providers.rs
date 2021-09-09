use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationSeconds};
use std::time::Duration;

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SuggestionProviderConfig {
    RemoteSettings(RemoteSettingsConfig),
    MemoryCache(MemoryCacheConfig),
    RedisCache(RedisCacheConfig),
    Multiplexer(MultiplexerConfig),
    Debug,
    WikiFruit,
    Null,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MultiplexerConfig {
    /// The multiplexed providers.
    pub providers: Vec<SuggestionProviderConfig>,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RedisCacheConfig {
    // /// The URL to connect to Redis at. Example: `redis://127.0.0.1/db`
    // #[serde_as(as = "crate::redis::AsConnectionInfo")]
    // pub url: redis::ConnectionInfo,
    /// The default time a cache entry will be valid for, if not specified by
    /// the inner provider.
    #[serde_as(as = "DurationSeconds")]
    #[serde(rename = "default_ttl_sec")]
    pub default_ttl: Duration,

    /// The default time to try and hold a lock for a response
    /// from the source on cache refresh/load.
    #[serde_as(as = "DurationSeconds")]
    #[serde(rename = "default_lock_timeout_sec")]
    pub default_lock_timeout: Duration,

    /// The cached provider.
    pub inner: Box<SuggestionProviderConfig>,
}

impl RedisCacheConfig {
    pub fn with_inner(inner: SuggestionProviderConfig) -> Self {
        Self {
            inner: Box::new(inner),
            ..Default::default()
        }
    }
}

impl Default for RedisCacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: Duration::from_secs(900), // 15 minutes
            default_lock_timeout: Duration::from_secs(3),
            inner: Box::new(SuggestionProviderConfig::Null),
        }
    }
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemoryCacheConfig {
    /// The default TTL to assign to a cache entry if the underlying provider does not provide one.
    #[serde_as(as = "DurationSeconds")]
    #[serde(rename = "default_ttl_sec")]
    pub default_ttl: Duration,

    /// The cleanup task will be run with a period equal to this setting. Any
    /// expired entries will be removed from the cache.
    #[serde_as(as = "DurationSeconds")]
    #[serde(rename = "cleanup_interval_sec")]
    pub cleanup_interval: Duration,

    /// While running the cleanup task, at most this many entries will be removed
    /// before cancelling the task. This should be used to limit the maximum
    /// amount of time the cleanup task takes.
    pub max_removed_entries: usize,

    /// The default TTL for in-memory locks to prevent multiple update requests from
    /// being fired at providers at the same time.
    #[serde_as(as = "DurationSeconds")]
    #[serde(rename = "default_lock_timeout_sec")]
    pub default_lock_timeout: Duration,

    /// The cached provider.
    pub inner: Box<SuggestionProviderConfig>,
}

impl MemoryCacheConfig {
    pub fn with_inner(inner: SuggestionProviderConfig) -> Self {
        Self {
            inner: Box::new(inner),
            ..Default::default()
        }
    }
}

impl Default for MemoryCacheConfig {
    fn default() -> Self {
        Self {
            default_ttl: Duration::from_secs(900),
            cleanup_interval: Duration::from_secs(300),
            max_removed_entries: 100_000,
            default_lock_timeout: Duration::from_secs(10),
            inner: Box::new(SuggestionProviderConfig::Null),
        }
    }
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct RemoteSettingsConfig {
    /// The collection to sync form.
    pub collection: String,
}

impl Default for RemoteSettingsConfig {
    fn default() -> Self {
        Self {
            collection: "quicksuggest".to_string(),
        }
    }
}
