//! Tools to manager providers.

use anyhow::Result;
use async_recursion::async_recursion;
use merino_adm::remote_settings::RemoteSettingsSuggester;
use merino_cache::{MemoryCacheSuggester, RedisCacheSuggester};
use merino_settings::{providers::SuggestionProviderConfig, Settings};
use merino_suggest::{
    DebugProvider, FixedProvider, Multi, NullProvider, SuggestionProvider, TimeoutProvider,
    WikiFruit,
};
use tokio::sync::OnceCell;
use tracing_futures::Instrument;

/// The SuggestionProvider stored in Actix's app_data.
pub struct SuggestionProviderRef(OnceCell<merino_suggest::Multi>);

impl SuggestionProviderRef {
    /// Make a new slot to hold providers. Does not initialize any providers.
    pub fn new() -> Self {
        Self(OnceCell::new())
    }

    /// Get the provider, or create a new one if it doesn't exist.
    pub async fn get_or_try_init(&self, settings: &Settings) -> Result<&Multi> {
        let setup_span = tracing::info_span!("suggestion_provider_setup");
        self.0
            .get_or_try_init(|| {
                async {
                    tracing::info!(
                        r#type = "web.configuring-suggesters",
                        "Setting up suggestion providers"
                    );

                    let mut providers: Vec<Box<dyn SuggestionProvider>> =
                        Vec::with_capacity(settings.suggestion_providers.len());
                    for config in settings.suggestion_providers.values() {
                        providers.push(make_provider_tree(settings, config).await?);
                    }

                    let multi = merino_suggest::Multi::new(providers);
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
) -> Result<Box<dyn SuggestionProvider>> {
    let provider: Box<dyn SuggestionProvider> = match config {
        SuggestionProviderConfig::RemoteSettings(rs_config) => {
            RemoteSettingsSuggester::new_boxed(settings, rs_config).await?
        }

        SuggestionProviderConfig::MemoryCache(memory_config) => {
            let inner = make_provider_tree(settings, memory_config.inner.as_ref()).await?;
            MemoryCacheSuggester::new_boxed(memory_config, inner)
        }

        SuggestionProviderConfig::RedisCache(redis_config) => {
            let inner = make_provider_tree(settings, redis_config.inner.as_ref()).await?;
            RedisCacheSuggester::new_boxed(settings, redis_config, inner).await?
        }

        SuggestionProviderConfig::Multiplexer(multi_config) => {
            let mut providers = Vec::new();
            for config in &multi_config.providers {
                providers.push(make_provider_tree(settings, config).await?);
            }
            Multi::new_boxed(providers)
        }

        SuggestionProviderConfig::Timeout(timeout_config) => {
            let inner = make_provider_tree(settings, timeout_config.inner.as_ref()).await?;
            TimeoutProvider::new_boxed(timeout_config, inner)
        }

        SuggestionProviderConfig::Fixed(fixed_config) => {
            FixedProvider::new_boxed(settings, fixed_config)?
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
        let provider_tree = make_provider_tree(&settings, &config).await?;
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

        let provider_tree = make_provider_tree(&settings, &config).await?;
        assert_eq!(
            provider_tree.name(),
            "Multi(NullProvider, RedisCache(MemoryCache(WikiFruit)), NullProvider)"
        );
        Ok(())
    }
}
