//! Tools to manager providers.

use anyhow::{Context, Result};
use cadence::StatsdClient;
use merino_settings::Settings;
use merino_settings::SuggestionProviderConfig;
use merino_suggest_providers::make_provider_tree;
use merino_suggest_providers::reconfigure_provider_tree;
use merino_suggest_providers::IdMulti;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock as TokioRwLock;

/// The SuggestionProvider stored in Actix's app_data.
#[derive(Clone)]
pub struct SuggestionProviderRef {
    /// The wrapped providers.
    pub provider: Arc<TokioRwLock<IdMulti>>,
    /// The metrics client used, to enable reconfiguration of providers.
    metrics_client: StatsdClient,
    /// The settings used to create the provider, to enable reconfiguration.
    settings: Settings,
}

impl SuggestionProviderRef {
    /// Initialize the suggestion providers
    /// # Errors
    /// If a provider fails to initialize.
    pub async fn init(settings: Settings, metrics_client: StatsdClient) -> Result<Self> {
        let mut idm = IdMulti::default();

        let _setup_span = tracing::info_span!("suggestion_provider_setup");
        tracing::info!(
            r#type = "web.configuring-suggesters",
            "Setting up suggestion providers"
        );

        for (name, config) in settings.suggestion_providers.clone() {
            idm.add_provider(
                name,
                make_provider_tree(settings.clone(), config, metrics_client.clone()).await?,
            );
        }

        Ok(Self {
            provider: Arc::new(TokioRwLock::new(idm)),
            metrics_client,
            settings,
        })
    }

    /// Reconfigure the reference providers with a new config.
    /// # Errors
    /// If a provider fails to reconfigure, or if there is a problem converting the provider configuration to the require format.
    pub async fn reconfigure(
        &self,
        suggestion_providers: HashMap<String, SuggestionProviderConfig>,
    ) -> Result<()> {
        let mut id_provider = self.provider.write().await;
        let type_erased_config = serde_json::to_value(suggestion_providers)
            .context("serialized context for provider reconfiguration")?;
        reconfigure_provider_tree(
            &mut *id_provider,
            self.settings.clone(),
            type_erased_config,
            self.metrics_client.clone(),
        )
        .await
        .context("reconfiguring providers")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::make_provider_tree;
    use anyhow::Result;
    use cadence::{SpyMetricSink, StatsdClient};
    use merino_settings::{
        providers::{
            MemoryCacheConfig, MultiplexerConfig, RedisCacheConfig, SuggestionProviderConfig,
        },
        Settings,
    };

    #[tokio::test]
    async fn test_providers_single() -> Result<()> {
        let settings = Settings::load_for_tests();
        let config = SuggestionProviderConfig::Null;
        let metrics_client = StatsdClient::from_sink("merino-test", SpyMetricSink::new().1);
        let provider_tree = make_provider_tree(settings, config, metrics_client).await?;
        assert_eq!(provider_tree.name(), "NullProvider");
        Ok(())
    }

    #[tokio::test]
    async fn test_providers_complex() -> Result<()> {
        let mut settings = Settings::load_for_tests();
        settings.debug = true;

        let config = SuggestionProviderConfig::Multiplexer(MultiplexerConfig {
            providers: vec![
                SuggestionProviderConfig::Null,
                SuggestionProviderConfig::RedisCache(RedisCacheConfig {
                    inner: Box::new(SuggestionProviderConfig::MemoryCache(MemoryCacheConfig {
                        inner: Box::new(SuggestionProviderConfig::WikiFruit),
                        ..Default::default()
                    })),
                    ..Default::default()
                }),
                SuggestionProviderConfig::Null,
            ],
        });

        let metrics_client = StatsdClient::from_sink("merino-test", SpyMetricSink::new().1);
        let provider_tree = make_provider_tree(settings, config, metrics_client).await?;
        assert_eq!(
            provider_tree.name(),
            "Multi(NullProvider, RedisCache(MemoryCache(WikiFruit)), NullProvider)"
        );
        Ok(())
    }
}
