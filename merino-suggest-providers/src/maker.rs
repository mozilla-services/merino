//! Tools to build providers from configuration.

use crate::{
    ClientVariantFilterProvider, DebugProvider, FixedProvider, KeywordFilterProvider, Multi,
    NullProvider, StealthProvider, TimeoutProvider, WikiFruit,
};
use anyhow::Result;
use async_recursion::async_recursion;
use cadence::StatsdClient;
use merino_adm::remote_settings::RemoteSettingsSuggester;
use merino_cache::{MemoryCacheSuggester, RedisCacheSuggester};
use merino_settings::{providers::SuggestionProviderConfig, Settings};
use merino_suggest_traits::{SetupError, SuggestionProvider};

/// Recursive helper to build a tree of providers.
#[async_recursion]
pub async fn make_provider_tree(
    settings: Settings,
    config: SuggestionProviderConfig,
    metrics_client: StatsdClient,
) -> Result<Box<dyn SuggestionProvider>, SetupError> {
    let provider: Box<dyn SuggestionProvider> = match config {
        SuggestionProviderConfig::RemoteSettings(rs_config) => {
            RemoteSettingsSuggester::new_boxed(settings, rs_config, metrics_client.clone()).await?
        }

        SuggestionProviderConfig::MemoryCache(memory_config) => {
            let inner = make_provider_tree(
                settings,
                memory_config.inner.as_ref().clone(),
                metrics_client.clone(),
            )
            .await?;
            MemoryCacheSuggester::new_boxed(memory_config, inner, metrics_client.clone())
        }

        SuggestionProviderConfig::RedisCache(redis_config) => {
            let inner = make_provider_tree(
                settings.clone(),
                redis_config.inner.as_ref().clone(),
                metrics_client.clone(),
            )
            .await?;
            RedisCacheSuggester::new_boxed(settings, redis_config, metrics_client.clone(), inner)
                .await?
        }

        SuggestionProviderConfig::Multiplexer(multi_config) => {
            let mut providers = Vec::new();
            for config in multi_config.providers {
                providers.push(
                    make_provider_tree(settings.clone(), config, metrics_client.clone()).await?,
                );
            }
            Multi::new_boxed(providers)
        }

        SuggestionProviderConfig::Timeout(timeout_config) => {
            let inner = make_provider_tree(
                settings,
                timeout_config.inner.as_ref().clone(),
                metrics_client,
            )
            .await?;
            TimeoutProvider::new_boxed(timeout_config, inner)
        }

        SuggestionProviderConfig::Fixed(fixed_config) => {
            FixedProvider::new_boxed(settings, fixed_config)?
        }

        SuggestionProviderConfig::KeywordFilter(filter_config) => {
            let inner = make_provider_tree(
                settings,
                filter_config.inner.as_ref().clone(),
                metrics_client.clone(),
            )
            .await?;
            KeywordFilterProvider::new_boxed(filter_config, inner, metrics_client.clone())?
        }

        SuggestionProviderConfig::Stealth(filter_config) => {
            let inner = make_provider_tree(
                settings,
                filter_config.inner.as_ref().clone(),
                metrics_client,
            )
            .await?;
            StealthProvider::new_boxed(inner)
        }

        SuggestionProviderConfig::ClientVariantSwitch(filter_config) => {
            let matching_provider = make_provider_tree(
                settings.clone(),
                filter_config.matching_provider.as_ref().clone(),
                metrics_client.clone(),
            )
            .await?;
            let default_provider = make_provider_tree(
                settings.clone(),
                filter_config.default_provider.as_ref().clone(),
                metrics_client.clone(),
            )
            .await?;
            ClientVariantFilterProvider::new_boxed(
                matching_provider,
                default_provider,
                filter_config.client_variant.clone(),
            )
        }

        SuggestionProviderConfig::Debug => DebugProvider::new_boxed(settings)?,
        SuggestionProviderConfig::WikiFruit => WikiFruit::new_boxed(settings)?,
        SuggestionProviderConfig::Null => Box::new(NullProvider),
    };
    Ok(provider)
}
