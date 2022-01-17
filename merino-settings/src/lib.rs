//! # Merino Settings
//!
//! The top level settings struct is [Settings]. If you are looking for
//! documentation about the settings that can be set, start there.
//!
//! Configuration is specified in several ways, with later methods overriding earlier ones.
//!
//! 1. A base configuration checked into the repository, in `config/base.yaml`.
//!    This provides the default values for most settings.
//! 2. Per-environment configuration files in the `config` directory. The
//!    environment is selected using the environment variable `MERINO_ENV`. The
//!    settings for that environment are then loaded from `config/${env}.yaml`, if
//!    it exists. The default environment is "development". A "production"
//!    environment is also provided.
//! 3. A local configuration file not checked into the repository, at
//!    `config/local.yaml`. This file is in `.gitignore` and is safe to use for
//!    local configuration and secrets if desired.
//! 4. Environment variables that begin with `MERINO_` and use `__` as a level
//!    separator. For example, `Settings::http::workers` can be controlled from the
//!    environment variable `MERINO_HTTP__WORKERS`.
//!
//! Tests should use `Settings::load_for_test` which only reads from
//! `config/base.yaml`, `config/test.yaml`, and `config/local_test.yaml` (if it
//! exists). It does not read from environment variables.
//!
//! Configuration files are canonically YAML files. However, any format supported
//! by the [config] crate can be used, including JSON and TOML. To choose another
//! format, simply use a different extension for your file, like
//! `config/local.toml`.

mod logging;
pub mod providers;
mod redis;

pub use logging::{LogFormat, LoggingSettings};

use anyhow::{Context, Result};
use config::{Config, Environment, File};
use http::Uri;
use sentry::internals::Dsn;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::{collections::HashMap, net::SocketAddr, path::PathBuf, str::FromStr};

use crate::providers::{SuggestionProviderConfig, SuggestionProviderSettings};

/// Top level settings object for Merino.
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    /// The environment Merino is running in. Should only be set with the
    /// `MERINO_ENV` environment variable.
    pub env: String,

    /// Enable additional features to debug the application. This should not be
    /// set to true in production environments.
    pub debug: bool,

    /// Settings for the HTTP server.
    pub http: HttpSettings,

    /// Providers to use to generate suggestions
    #[serde(default)]
    pub suggestion_providers: HashMap<String, SuggestionProviderConfig>,

    /// Logging settings.
    pub logging: LoggingSettings,

    /// Metrics settings.
    pub metrics: MetricsSettings,

    /// Settings for error reporting via Sentry.
    pub sentry: SentrySettings,

    /// URL to redirect curious users to, that explains what this service is.
    /// Preferable a public wiki page. Optional.
    #[serde_as(as = "Option<DisplayFromStr>")]
    pub public_documentation: Option<Uri>,

    /// Settings for connecting to Redis.
    pub redis: RedisSettings,

    /// Settings for connecting to Remote Settings.
    pub remote_settings: RemoteSettingsGlobalSettings,

    /// Settings to use when determining the location associated with requests.
    pub location: LocationSettings,

    /// If on, log the entire suggestion request object as a part of the
    /// tracing log, including the search query. When the setting is
    /// off, the suggest request object should be logged, but the
    /// search query should be blank.
    pub log_full_request: bool,
}

/// Settings for the HTTP server.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HttpSettings {
    /// The host and port to listen on, such as "127.0.0.1:8080" or "0.0.0.0:80".
    pub listen: SocketAddr,

    /// The number of workers to use. Optional. If no value is provided, the
    /// number of logical cores will be used.
    pub workers: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheType {
    None,
    Redis,
    Memory,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdmRsSettings {
    /// Whether this provider should be active.
    pub enabled: bool,

    /// The path, relative or absolute, to where to store Remote Settings data.
    pub storage_path: PathBuf,

    /// The server to sync from. If no value is provided, a default is provided
    /// by the remote settings client.
    pub server: Option<String>,

    /// The collection to sync form.
    pub collection: String,

    /// Which cache, if any, to use with this provider.
    pub cache: CacheType,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WikiFruitSettings {
    /// Whether this provider should be active.
    pub enabled: bool,

    /// Which cache, if any, to use with this provider.
    pub cache: CacheType,
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RedisSettings {
    /// The URL to connect to Redis at. Example: `redis://127.0.0.1/db`
    #[serde_as(as = "crate::redis::AsConnectionInfo")]
    pub url: ::redis::ConnectionInfo,
}

impl RedisSettings {
    #[must_use]
    pub fn redacted_url(&self) -> String {
        match (&self.url.username, &self.url.passwd) {
            (Some(username), Some(_)) => {
                format!(
                    "redis://{}:<PASSWORD>@{}/{}",
                    username, self.url.addr, self.url.db
                )
            }
            (Some(username), None) => {
                format!("redis://{}@{}/{}", username, self.url.addr, self.url.db)
            }
            (None, Some(_password)) => {
                format!("redis://:<PASSWORD>@{}/{}", self.url.addr, self.url.db)
            }
            (None, None) => format!("redis://{}/{}", self.url.addr, self.url.db),
        }
    }
}

#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RemoteSettingsGlobalSettings {
    /// The server to sync from, including the protocol and port, but not including a trailing slash.
    ///
    /// ## Examples
    /// - `http://127.0.0.1`
    /// - `https://firefox.settings.services.mozilla.com`
    pub server: String,

    /// The bucket to use for Remote Settings providers, if not overridden by
    /// the provider.
    pub default_bucket: String,

    /// The collection to use for Remote Settings providers, if not overridden
    /// by the provider.
    pub default_collection: String,

    /// Only used for integration tests.
    /// This field populates the mock returned remote settings collection
    /// from inside the `merino_test_macro!(|settings| {...}) call.
    // note `#[cfg(test)]` will cause a compile error as the
    // `merino_test` proc_macro will not be able to find this
    // field.
    pub test_changes: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LocationSettings {
    /// The location of the maxmind database to use to determine IP location. If
    /// not specified, location information will not be calculated.
    pub maxmind_database: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricsSettings {
    /// The host/IP address to send metrics to, such as "127.0.0.1" or "metrics.local".
    pub sink_host: String,

    /// The port to send metrics to.
    pub sink_port: u16,

    /// Maximum size in kilobytes that the metrics queue can grow to before
    /// locale metrics start to be dropped.
    pub max_queue_size_kb: usize,
}

/// Settings for the error and event reporting system Sentry.
///
/// Uses an enum to maintain invariants. In yaml or environment variable
/// configs, set using one of these patterns:
///
/// * mode=release, dsn=https://..., env=stage
/// * mode=server_debug, dsn=https://..., who=your_name
/// * mode=local_debug
/// * mode=disabled
///
/// In local_debug mode, events will be logged, but the DSN setting will be
/// ignored.  It will be set to a testing value as recommended by Sentry's docs.
///
/// In server_debug mode, you should configure the DSN to point to a testing
/// project on a real Sentry instance.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum SentrySettings {
    Release { dsn: Dsn, env: String },
    ServerDebug { dsn: Dsn, who: String },
    LocalDebug,
    Disabled,
}

impl SentrySettings {
    /// Get the configured DSN.
    pub fn dsn(&self) -> Option<Dsn> {
        match self {
            SentrySettings::Release { dsn, .. } | SentrySettings::ServerDebug { dsn, .. } => {
                Some(dsn.clone())
            }
            SentrySettings::LocalDebug => Some(
                Dsn::from_str("https://public@example.com/1").expect("Bug: debug DSN is not valid"),
            ),
            SentrySettings::Disabled => None,
        }
    }

    /// Check if the Sentry settings are in debug mode
    pub fn debug(&self) -> bool {
        match self {
            SentrySettings::Release { .. } => false,
            SentrySettings::ServerDebug { .. } => true,
            SentrySettings::LocalDebug => true,
            SentrySettings::Disabled => false,
        }
    }

    pub fn env(&self) -> &str {
        match self {
            SentrySettings::Release { env, .. } => env.as_str(),
            SentrySettings::ServerDebug { who, .. } => who.as_str(),
            SentrySettings::LocalDebug => "debug",
            SentrySettings::Disabled => "disabled",
        }
    }
}

impl Settings {
    /// Load settings from configuration files and environment variables.
    ///
    /// # Errors
    /// If any of the configured values are invalid, or if any of the required
    /// configuration files are missing.
    pub fn load() -> Result<Self> {
        let mut s = Config::new();

        // Start off with the base config.
        s.merge(File::with_name("./config/base"))
            .context("loading base config")?;

        // Merge in an environment specific config.
        let merino_env = std::env::var("MERINO_ENV").unwrap_or_else(|_| "development".to_string());
        s.set("env", merino_env.as_str())
            .context("loading merino environment name")?;
        s.merge(File::with_name(&format!("config/{}", s.get::<String>("env")?)).required(false))
            .context("loading environment config")?;

        // Add a local configuration file that is `.gitignore`ed.
        s.merge(File::with_name("config/local").required(false))
            .context("loading local config overrides")?;

        // Add environment variables that start with "MERINO_" and have "__" to
        // separate levels. For example, `MERINO_HTTP__LISTEN` maps to
        // `Settings::http::listen`.
        s.merge(Environment::default().prefix("MERINO").separator("__"))
            .context("merging config")?;

        let mut settings: Settings =
            serde_path_to_error::deserialize(s).context("Deserializing settings")?;
        let provider_settings = SuggestionProviderSettings::load()?;
        settings.suggestion_providers = provider_settings.0;

        Ok(settings)
    }

    /// Load settings from configuration files for tests.
    pub fn load_for_tests() -> Self {
        let mut s = Config::new();

        // Start off with the base config.
        s.merge(File::with_name("../config/base"))
            .expect("Could not load base settings");

        // Merge in test specific config.
        s.set("env", "test").expect("Could not set env for tests");
        s.merge(File::with_name("../config/test"))
            .expect("Could not load test settings");

        // Add a local configuration file that is `.gitignore`ed.
        s.merge(File::with_name("../config/local_test").required(false))
            .expect("Could not load local settings for tests");

        let mut settings: Settings = s.try_into().expect("Could not convert settings");
        let provider_settings = SuggestionProviderSettings::load_for_tests();
        settings.suggestion_providers = provider_settings.0;

        settings
    }
}

#[cfg(test)]
mod tests {
    use crate::Settings;
    use anyhow::{Context, Result};
    use config::{Config, File};
    use parameterized::parameterized;

    fn load_config_files(files: &[&str]) -> Result<Config> {
        let mut config = Config::new();
        for f in files {
            config
                .merge(File::with_name(f))
                .context(format!("Loading config {}", f))?;
        }

        Ok(config)
    }

    #[parameterized(config_name = {"ci", "development", "production", "test"})]
    fn config_loads(config_name: &str) {
        let mut config =
            load_config_files(&["../config/base", &format!("../config/{}", config_name)])
                .expect("could not read config files");

        // config is a required field that should never be set in the provided files.
        assert!(config.get_str("env").is_err());
        config
            .set("env", "config_name")
            .expect("Could not set value");

        // special case: prod needs sentry manually configured
        if config_name == "production" {
            config
                .set("sentry.dsn", "https://public@example.com/1")
                .expect("Could not set value")
                .set("sentry.env", "test")
                .expect("Could not set value");
        }

        let settings = config.try_into::<Settings>();
        if let Err(err) = &settings {
            println!("Problem while testing {} config: {}", config_name, err);
        }
        assert!(settings.is_ok());
    }
}
