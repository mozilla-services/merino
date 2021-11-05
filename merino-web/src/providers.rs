//! Tools to manager providers.

use anyhow::Result;
use async_recursion::async_recursion;
use cadence::StatsdClient;
use merino_adm::remote_settings::RemoteSettingsSuggester;
use merino_cache::{MemoryCacheSuggester, RedisCacheSuggester};
use merino_settings::{providers::SuggestionProviderConfig, Settings};
use merino_suggest::{
    DebugProvider, FixedProvider, IdMulti, KeywordFilterProvider, Multi, NullProvider,
    StealthProvider, SuggestionProvider, TimeoutProvider, WikiFruit,
};
use tokio::sync::OnceCell;
use tracing_futures::Instrument;

/// The SuggestionProvider stored in Actix's app_data.
pub struct SuggestionProviderRef(OnceCell<merino_suggest::IdMulti>);

impl SuggestionProviderRef {
    /// Make a new slot to hold providers. Does not initialize any providers.
    pub fn new() -> Self {
        Self(OnceCell::new())
    }

    /// Get the provider, or create a new one if it doesn't exist.
    pub async fn get_or_try_init(
        &self,
        settings: &Settings,
        metrics_client: &StatsdClient,
    ) -> Result<&IdMulti> {
        let setup_span = tracing::info_span!("suggestion_provider_setup");
        self.0
            .get_or_try_init(|| {
                async {
                    tracing::info!(
                        r#type = "web.configuring-suggesters",
                        "Setting up suggestion providers"
                    );

                    let mut multi = merino_suggest::IdMulti::default();
                    // let mut providers: Vec<Box<dyn SuggestionProvider>> =
                    //     Vec::with_capacity(settings.suggestion_providers.len());
                    for (name, config) in &settings.suggestion_providers {
                        multi.add_provider(
                            name,
                            make_provider_tree(settings, config, metrics_client).await?,
                        );
                    }

                    Ok(multi)
                }
                .instrument(setup_span)
            })
            .await
    }
}

/// Recursive helper to build a tree of providers.
#[async_recursion]
async fn make_provider_tree(
    settings: &Settings,
    config: &SuggestionProviderConfig,
    metrics_client: &StatsdClient,
) -> Result<Box<dyn SuggestionProvider>> {
    let provider: Box<dyn SuggestionProvider> = match config {
        SuggestionProviderConfig::RemoteSettings(rs_config) => {
            RemoteSettingsSuggester::new_boxed(settings, rs_config, metrics_client.clone()).await?
        }

        SuggestionProviderConfig::MemoryCache(memory_config) => {
            let inner =
                make_provider_tree(settings, memory_config.inner.as_ref(), metrics_client).await?;
            MemoryCacheSuggester::new_boxed(memory_config, inner, metrics_client.clone())
        }

        SuggestionProviderConfig::RedisCache(redis_config) => {
            let inner =
                make_provider_tree(settings, redis_config.inner.as_ref(), metrics_client).await?;
            RedisCacheSuggester::new_boxed(settings, redis_config, metrics_client.clone(), inner)
                .await?
        }

        SuggestionProviderConfig::Multiplexer(multi_config) => {
            let mut providers = Vec::new();
            for config in &multi_config.providers {
                providers.push(make_provider_tree(settings, config, metrics_client).await?);
            }
            Multi::new_boxed(providers)
        }

        SuggestionProviderConfig::Timeout(timeout_config) => {
            let inner =
                make_provider_tree(settings, timeout_config.inner.as_ref(), metrics_client).await?;
            TimeoutProvider::new_boxed(timeout_config, inner)
        }

        SuggestionProviderConfig::Fixed(fixed_config) => {
            FixedProvider::new_boxed(settings, fixed_config)?
        }

        SuggestionProviderConfig::KeywordFilter(filter_config) => {
            let inner =
                make_provider_tree(settings, filter_config.inner.as_ref(), metrics_client).await?;
            KeywordFilterProvider::new_boxed(
                filter_config.suggestion_blocklist.clone(),
                inner,
                metrics_client.clone(),
            )?
        }

        SuggestionProviderConfig::Stealth(filter_config) => {
            let inner =
                make_provider_tree(settings, filter_config.inner.as_ref(), metrics_client).await?;
            StealthProvider::new_boxed(inner)
        }

        SuggestionProviderConfig::Debug => DebugProvider::new_boxed(settings)?,
        SuggestionProviderConfig::WikiFruit => WikiFruit::new_boxed(settings)?,
        SuggestionProviderConfig::Null => Box::new(NullProvider),
    };
    Ok(provider)
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
