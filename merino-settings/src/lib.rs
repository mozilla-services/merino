//! # Merino Settings
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
//! 4. Environment variables that begin with `MERINO_` and have a separator for
//!    `__`. For example, `Settings::http::workers` can be controlled from the
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

pub use logging::LoggingSettings;

use config::{Config, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, path::PathBuf};

/// Top level settings object for Merino.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[doc(inline)]
pub struct Settings {
    /// The environment Merino is running in. Should only be set with the
    /// `MERINO_ENV` environment variable.
    pub env: String,

    /// Enable additional features to debug the application. This should not be
    /// set to true in production environments.
    pub debug: bool,

    /// Settings for the HTTP server.
    pub http: HttpSettings,

    /// Settings for adM integration.
    pub adm: AdmSettings,

    /// Logging settings.
    pub logging: LoggingSettings,
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

/// Settings for the adM integration.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdmSettings {
    /// Configuration for connection to Remote Settings to provide suggestions.
    pub remote_settings: AdmRemoteSettingsConfig,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AdmRemoteSettingsConfig {
    /// The path, relative or absolute, to where to store remote settings data.
    pub storage_path: PathBuf,

    /// The server to sync from. If no value is provided, a default is provided
    /// by the remote settings client.
    pub server: Option<String>,

    /// The collection to sync form.
    pub collection: String,
}

impl Settings {
    /// Load settings from configuration files and environment variables.
    ///
    /// # Errors
    /// If any of the configured values are invalid, or if any of the required
    /// configuration files are missing.
    pub fn load() -> Result<Self, ConfigError> {
        let mut s = Config::new();

        // Start off with the base config.
        s.merge(File::with_name("./config/base"))?;

        // Merge in an environment specific config.
        let merino_env = std::env::var("MERINO_ENV").unwrap_or_else(|_| "development".to_string());
        s.set("env", merino_env.as_str())?;
        s.merge(File::with_name(&format!("config/{}", s.get::<String>("env")?)).required(false))?;

        // Add a local configuration file that is `.gitignore`ed.
        s.merge(File::with_name("config/local").required(false))?;

        // Add environment variables that start with "MERINO_" and have "__" to
        // separate levels. For example, `MERINO_HTTP__LISTEN` maps to
        // `Settings::http::listen`.
        s.merge(Environment::default().prefix("MERINO").separator("__"))?;

        s.try_into()
    }

    /// Load settings from configuration files for tests.
    ///
    /// `changer` is
    pub fn load_for_tests<F: FnOnce(&mut Self)>(changer: F) -> Self {
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

        let mut config = s.try_into().expect("Could not convert settings");
        changer(&mut config);
        config
    }
}
