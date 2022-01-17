use anyhow::{Context, Result};
use config::{Config, File};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DurationMilliSeconds, DurationSeconds};
use std::collections::HashMap;
use std::time::Duration;

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SuggestionProviderConfig {
    RemoteSettings(RemoteSettingsConfig),
    MemoryCache(MemoryCacheConfig),
    RedisCache(RedisCacheConfig),
    Multiplexer(MultiplexerConfig),
    Timeout(TimeoutConfig),
    Fixed(FixedConfig),
    KeywordFilter(KeywordFilterConfig),
    Stealth(StealthConfig),
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
    #[must_use]
    pub fn with_inner(inner: SuggestionProviderConfig) -> Self {
        Self {
            inner: Box::new(inner),
            ..Self::default()
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
#[serde(default)]
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
    #[must_use]
    pub fn with_inner(inner: SuggestionProviderConfig) -> Self {
        Self {
            inner: Box::new(inner),
            ..Self::default()
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
    /// The Remote Settings bucket to read from. If `None`, the default from the
    /// global config will be used.
    pub bucket: Option<String>,

    /// The collection to sync form. If `None`, the default from the global
    /// config will be used.
    pub collection: Option<String>,

    /// The time between re-syncs of Remote Settings data.
    #[serde_as(as = "DurationSeconds")]
    #[serde(rename = "resync_interval_sec")]
    pub resync_interval: Duration,

    /// The score value to assign to suggestions. A float between 0.0 and 1.0 inclusive.
    pub suggestion_score: f32,
}

impl Default for RemoteSettingsConfig {
    fn default() -> Self {
        Self {
            bucket: None,
            collection: None,
            resync_interval: Duration::from_secs(60 * 60 * 3), // 3 hours
            suggestion_score: 0.3,
        }
    }
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct TimeoutConfig {
    #[serde_as(as = "DurationMilliSeconds")]
    #[serde(rename = "max_time_ms")]
    pub max_time: Duration,

    pub inner: Box<SuggestionProviderConfig>,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            max_time: Duration::from_millis(200),
            inner: Box::new(SuggestionProviderConfig::Null),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FixedConfig {
    /// The value to use in the title of the suggestion.
    pub value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct KeywordFilterConfig {
    /// A blocklist to filter suggestions coming providers.
    pub suggestion_blocklist: HashMap<String, String>,

    /// The filtered provider.
    pub inner: Box<SuggestionProviderConfig>,
}

impl Default for KeywordFilterConfig {
    fn default() -> Self {
        Self {
            suggestion_blocklist: HashMap::new(),
            inner: Box::new(SuggestionProviderConfig::Null),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct StealthConfig {
    /// The provider to run but not return data from.
    pub inner: Box<SuggestionProviderConfig>,
}

impl Default for StealthConfig {
    fn default() -> Self {
        Self {
            inner: Box::new(SuggestionProviderConfig::Null),
        }
    }
}

/// Settings for Merino suggestion providers.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SuggestionProviderSettings(pub HashMap<String, SuggestionProviderConfig>);

impl SuggestionProviderSettings {
    /// Load settings for suggestions providers.
    ///
    /// The organization of the provider configuration files is identical to the
    /// top level settings.
    ///
    /// Note that settings for suggestion providers cannot be configured via
    /// environment variables.
    ///
    /// # Errors
    /// If any of the configured values are invalid, or if any of the required
    /// configuration files are missing.
    pub fn load() -> Result<Self> {
        let mut s = Config::new();

        // Start off with the base config.
        s.merge(File::with_name("./config/providers/base"))
            .context("loading base config for suggestion providers")?;

        // Merge in an environment specific config.
        let merino_env = std::env::var("MERINO_ENV").unwrap_or_else(|_| "development".to_string());
        s.merge(File::with_name(&format!("./config/providers/{}", merino_env)).required(false))
            .context("loading environment config for suggestion providers")?;

        // Add a local configuration file that is `.gitignore`ed.
        s.merge(File::with_name("./config/providers/local").required(false))
            .context("loading local config for suggestion providers")?;

        serde_path_to_error::deserialize(s)
            .context("Deserializing settings for suggestion providers")
    }

    /// Load settings for suggestion providers from configuration files for tests.
    pub fn load_for_tests() -> Self {
        let mut s = Config::new();

        s.merge(File::with_name("../config/providers/test"))
            .expect("Could not load test settings for suggestion providers");

        // Add a local configuration file that is `.gitignore`ed.
        s.merge(File::with_name("../config/providers/local_test").required(false))
            .expect("Could not load local test settings for suggestion providers");

        s.try_into()
            .expect("Could not convert settings for suggestion providers")
    }
}

#[cfg(test)]
mod tests {
    use crate::{providers::SuggestionProviderConfig, Settings};
    use anyhow::{Context, Result};
    use config::{Config, File, Value};
    use serde_json::json;

    #[test]
    fn provider_defaults_are_optional() -> Result<()> {
        let mut config = Config::new();
        config.merge(File::with_name("../config/base"))?;
        config.set("env", "test")?;

        // Providers are allowed to have required fields, if there is no logical
        // default. If that's the case, make sure to add them here. Don't
        // provide any values for fields that are options.
        let value_json = json!({
            "memory_cache": { "type": "memory_cache" },
            "remote_settings": { "type": "remote_settings"},
            "redis_cache": { "type": "redis_cache"},
            "multiplexer": { "type": "multiplexer" },
            "debug": { "type": "debug"},
            "wiki_fruit": { "type": "wiki_fruit"},
            "null": { "type": "null"},
            "timeout": { "type": "timeout" },
            "fixed": { "type": "fixed", "value": "test suggestion" },
            "keyword_filter": { "type": "keyword_filter" },
            "stealth": { "type": "stealth" },
        });

        let value_config: Value = serde_json::from_value(value_json.clone())?;
        config.set("suggestion_providers", value_config)?;

        let settings = config
            .try_into::<Settings>()
            .context("could not convert settings")?;

        let mut found_providers = 0;
        for (id, provider) in settings.suggestion_providers {
            assert!(value_json.get(id).is_some());

            // This match clause helps ensure this test covers all providers. If
            // you have to add a case to this match, add it to `value_json`
            // above as well so it can be tested.
            found_providers += 1;
            assert!(
                match provider {
                    SuggestionProviderConfig::RemoteSettings(_)
                    | SuggestionProviderConfig::MemoryCache(_)
                    | SuggestionProviderConfig::RedisCache(_)
                    | SuggestionProviderConfig::Multiplexer(_)
                    | SuggestionProviderConfig::Timeout(_)
                    | SuggestionProviderConfig::KeywordFilter(_)
                    | SuggestionProviderConfig::Stealth(_)
                    | SuggestionProviderConfig::Debug
                    | SuggestionProviderConfig::WikiFruit
                    | SuggestionProviderConfig::Fixed(_)
                    | SuggestionProviderConfig::Null => true,
                },
                "all providers should be recognized"
            );
        }
        // Likewise, if this number needs to change, make sure to update the rest of the test.
        assert_eq!(found_providers, 11);

        Ok(())
    }
}
