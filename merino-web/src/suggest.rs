//! Web handlers for the suggestions API.

use crate::{errors::HandlerError, extractors::SuggestionRequestWrapper};
use actix_web::{
    get,
    web::{self, Data, ServiceConfig},
    HttpResponse,
};
use anyhow::Result;
use cadence::{CountedExt, Histogrammed, StatsdClient};
use merino_adm::remote_settings::RemoteSettingsSuggester;
use merino_cache::{MemoryCacheSuggester, RedisCacheSuggester};
use merino_settings::{CacheType, Settings};
use merino_suggest::{DebugProvider, Suggestion, SuggestionProvider, WikiFruit};
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
    /// Query Paramater for client_variants
    client_variants: Vec<String>,
}
/// Customizes the output format of [`Suggestion`].
#[derive(Debug)]
struct SuggestionWrapper<'a>(&'a Suggestion);

/// Suggest content in response to the queried text.
#[get("")]
#[tracing::instrument(skip(suggestion_request, provider, settings))]
async fn suggest<'a>(
    SuggestionRequestWrapper(suggestion_request): SuggestionRequestWrapper<'a>,
    provider: Data<SuggestionProviderRef<'a>>,
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
            HandlerError::Internal
        })?;

    let response = provider
        .suggest(suggestion_request)
        .await
        .map_err(|error| {
            tracing::error!(%error, r#type="web.suggest.error", "Error providing suggestions");
            HandlerError::Internal
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
struct SuggestionProviderRef<'a>(OnceCell<merino_suggest::Multi<'a>>);

impl<'a> SuggestionProviderRef<'a> {
    /// Get the provider, or create a new one if it doesn't exist.
    async fn get_or_try_init(
        &self,
        settings: &Settings,
    ) -> anyhow::Result<&merino_suggest::Multi<'a>> {
        let setup_span = tracing::info_span!("suggestion_provider_setup");
        self.0
            .get_or_try_init(|| {
                async {
                    let settings = settings;
                    tracing::info!(
                        r#type = "web.configuring-suggesters",
                        "Setting up suggestion providers"
                    );

                    /// The number of providers we expect to have, so we usually
                    /// don't have to re-allocate the vec.
                    const NUM_PROVIDERS: usize = 3;
                    let mut providers: Vec<Box<dyn SuggestionProvider + Send + Sync>> =
                        Vec::with_capacity(NUM_PROVIDERS);

                    if settings.providers.wiki_fruit.enabled {
                        let wikifruit = WikiFruit::new_boxed(settings)?;
                        providers.push(match settings.providers.wiki_fruit.cache {
                            CacheType::None => wikifruit,
                            CacheType::Redis => {
                                RedisCacheSuggester::new_boxed(settings, *wikifruit).await?
                            }
                            CacheType::Memory => {
                                MemoryCacheSuggester::new_boxed(settings, *wikifruit)
                            }
                        });
                    }

                    if settings.providers.adm_rs.enabled {
                        let adm_rs = RemoteSettingsSuggester::new_boxed(settings).await?;
                        providers.push(match settings.providers.adm_rs.cache {
                            CacheType::None => adm_rs,
                            CacheType::Redis => {
                                RedisCacheSuggester::new_boxed(settings, *adm_rs).await?
                            }
                            CacheType::Memory => MemoryCacheSuggester::new_boxed(settings, *adm_rs),
                        });
                    }

                    if settings.providers.enable_debug_provider {
                        providers.push(DebugProvider::new_boxed(settings)?);
                    }

                    let multi = merino_suggest::Multi::new(providers);
                    Ok(multi)
                }
                .instrument(setup_span)
            })
            .await
    }
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
