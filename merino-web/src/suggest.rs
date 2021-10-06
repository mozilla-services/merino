//! Web handlers for the suggestions API.

use crate::{errors::HandlerError, extractors::SuggestionRequestWrapper};
use actix_web::{
    get,
    web::{self, Data, ServiceConfig},
    HttpResponse,
};
use anyhow::Result;
use async_recursion::async_recursion;
use cadence::{CountedExt, Histogrammed, StatsdClient};
use merino_adm::remote_settings::RemoteSettingsSuggester;
use merino_cache::{MemoryCacheSuggester, RedisCacheSuggester};
use merino_settings::{providers::SuggestionProviderConfig, Settings};
use merino_suggest::{
    DebugProvider, Multi, NullProvider, Suggestion, SuggestionProvider, TimeoutProvider, WikiFruit,
};
use serde::{Deserialize, Serialize};
use serde_with::{rust::StringWithSeparator, serde_as, CommaSeparator};
use tokio::sync::OnceCell;
use tracing_futures::Instrument;

/// Configure a route to use the Suggest service.
pub fn configure(config: &mut ServiceConfig) {
    config
        .app_data(Data::new(SuggestionProviderRef(OnceCell::new())))
        .service(suggest);
}

/// The response the API generates.
#[derive(Debug, Serialize)]
struct SuggestResponse<'a> {
    /// A list of suggestions from the service.
    suggestions: Vec<SuggestionWrapper<'a>>,
    /// A list of taken from the request query
    client_variants: Vec<String>,
    /// An empty list
    server_variants: Vec<String>,
}
/// Query parameters
#[serde_as]
#[derive(Debug, Deserialize)]
struct SuggestQueryParameters {
    #[serde_as(as = "StringWithSeparator::<CommaSeparator, String>")]
    #[serde(default)]
    /// Query Parameter for client_variants
    client_variants: Vec<String>,
}
/// Customizes the output format of [`Suggestion`].
#[derive(Debug)]
struct SuggestionWrapper<'a>(&'a Suggestion);

/// Suggest content in response to the queried text.
#[get("")]
#[tracing::instrument(skip(suggestion_request, provider, metrics_client, settings))]
async fn suggest(
    SuggestionRequestWrapper(suggestion_request): SuggestionRequestWrapper,
    provider: Data<SuggestionProviderRef>,
    settings: Data<Settings>,
    metrics_client: Data<StatsdClient>,
    query_parameters: web::Query<SuggestQueryParameters>,
) -> Result<HttpResponse, HandlerError> {
    let provider = provider
        .get_or_try_init(settings.as_ref())
        .await
        .map_err(|error| {
            tracing::error!(
                ?error,
                r#type = "web.suggest.setup-error",
                "suggester error"
            );
            HandlerError::internal()
        })?;

    let response = provider
        .suggest(suggestion_request)
        .await
        .map_err(|error| {
            tracing::error!(%error, r#type="web.suggest.error", "Error providing suggestions");
            HandlerError::internal()
        })?;

    tracing::debug!(
        r#type = "web.suggest.provided-count",
        suggestion_count = response.suggestions.len(),
        "Providing suggestions"
    );
    metrics_client
        .histogram("request.suggestion-per", response.suggestions.len() as u64)
        .ok();

    for client_variant in &query_parameters.client_variants {
        metrics_client
            .incr(&format!("client_variants.{}", client_variant))
            .ok();
    }

    let res = HttpResponse::Ok()
        .append_header(("X-Cache", response.cache_status.to_string()))
        .json(SuggestResponse {
            suggestions: response.suggestions.iter().map(SuggestionWrapper).collect(),
            client_variants: query_parameters.client_variants.clone(),
            server_variants: Vec::new(),
        });

    Ok(res)
}

/// The SuggestionProvider stored in Actix's app_data.
struct SuggestionProviderRef(OnceCell<merino_suggest::Multi>);

impl SuggestionProviderRef {
    /// Get the provider, or create a new one if it doesn't exist.
    async fn get_or_try_init(&self, settings: &Settings) -> anyhow::Result<&merino_suggest::Multi> {
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

        SuggestionProviderConfig::Debug => DebugProvider::new_boxed(settings)?,
        SuggestionProviderConfig::WikiFruit => WikiFruit::new_boxed(settings)?,
        SuggestionProviderConfig::Null => Box::new(NullProvider),
    };
    Ok(provider)
}

/// A mapper from the internal schema used by merino-suggest to the expected API.
impl<'a> Serialize for SuggestionWrapper<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        #[allow(clippy::missing_docs_in_private_items)]
        struct Generated<'a> {
            block_id: u32,
            full_keyword: &'a str,
            title: &'a str,
            url: String,
            impression_url: String,
            click_url: String,
            provider: &'a str,
            is_sponsored: bool,
            icon: String,
            advertiser: &'a str,
            score: f32,
        }

        let provider = &self.0.provider;
        let generated = Generated {
            block_id: self.0.id,
            full_keyword: &self.0.full_keyword,
            title: &self.0.title,
            url: self.0.url.to_string(),
            impression_url: self.0.impression_url.to_string(),
            click_url: self.0.click_url.to_string(),
            provider,
            is_sponsored: self.0.is_sponsored,
            icon: self.0.icon.to_string(),
            advertiser: provider,
            score: self.0.score.into(),
        };

        generated.serialize(serializer)
    }
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
