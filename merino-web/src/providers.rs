//! Tools to manager providers.

use std::sync::Arc;

use anyhow::Result;
use cadence::StatsdClient;
use merino_settings::Settings;
use merino_suggest_providers::make_provider_tree;
use merino_suggest_providers::IdMulti;

/// The SuggestionProvider stored in Actix's app_data.
#[derive(Clone)]
pub struct SuggestionProviderRef(pub Arc<IdMulti>);

impl SuggestionProviderRef {
    /// initialize the suggestion providers
    pub async fn init(settings: &Settings, metrics_client: &StatsdClient) -> Result<Self> {
        let mut idm = IdMulti::default();

        let _setup_span = tracing::info_span!("suggestion_provider_setup");
        tracing::info!(
            r#type = "web.configuring-suggesters",
            "Setting up suggestion providers"
        );

        for (name, config) in &settings.suggestion_providers {
            idm.add_provider(
                name,
                make_provider_tree(settings, config, metrics_client).await?,
            );
        }

        Ok(Self(Arc::new(idm)))
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
        let provider_tree = make_provider_tree(&settings, &config, &metrics_client).await?;
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
        let provider_tree = make_provider_tree(&settings, &config, &metrics_client).await?;
        assert_eq!(
            provider_tree.name(),
            "Multi(NullProvider, RedisCache(MemoryCache(WikiFruit)), NullProvider)"
        );
        Ok(())
    }
}
