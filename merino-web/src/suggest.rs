//! Web handlers for the suggestions API.

use crate::{errors::HandlerError, extractors::SuggestionRequestWrapper};
use actix_web::{
    get,
    web::{Data, ServiceConfig},
    HttpResponse,
};
use anyhow::Result;
use merino_adm::remote_settings::RemoteSettingsSuggester;
use merino_cache::{MemoryCacheSuggester, RedisCacheSuggester};
use merino_settings::{CacheType, Settings};
use merino_suggest::{DebugProvider, Suggestion, SuggestionProvider, WikiFruit};
use serde::Serialize;
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
    suggestions: &'a [Suggestion],
}

/// Suggest content in response to the queried text.
#[get("")]
#[tracing::instrument(skip(suggestion_request, provider, settings))]
async fn suggest<'a>(
    SuggestionRequestWrapper(suggestion_request): SuggestionRequestWrapper<'a>,
    provider: Data<SuggestionProviderRef<'a>>,
    settings: Data<Settings>,
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

    let res = HttpResponse::Ok()
        .append_header(("X-Cache", response.cache_status.to_string()))
        .json(SuggestResponse {
            suggestions: &response.suggestions,
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
