//! Web handlers for the suggestions API.

use crate::errors::HandlerError;
use actix_web::{
    get,
    web::{Data, Query, ServiceConfig},
    HttpResponse,
};
use anyhow::Result;
use merino_settings::Settings;
use merino_suggest::{Suggestion, SuggestionProvider, WikiFruit};
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;
use tracing_futures::Instrument;

/// Configure a route to use the Suggest service.
pub fn configure(config: &mut ServiceConfig) {
    config
        .data(SuggestionProviderRef(OnceCell::new()))
        .service(suggest);
}

/// A query passed to the API.
#[derive(Debug, Deserialize)]
struct SuggestQuery {
    /// The query to generate suggestions for.
    q: String,
}

/// The response the API generates.
#[derive(Debug, Serialize)]
struct SuggestResponse {
    /// A list of suggestions from the service.
    suggestions: Vec<Suggestion>,
}

/// Suggest content in response to the queried text.
#[get("")]
#[tracing::instrument(skip(query, provider, settings))]
async fn suggest<'a>(
    query: Query<SuggestQuery>,
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

    let suggestions: Vec<Suggestion> = provider.suggest(&query.q).await.map_err(|error| {
        tracing::error!(%error, r#type="web.suggest.error", "Error providing suggestions");
        HandlerError::Internal
    })?;

    tracing::debug!(
        r#type = "web.suggest.provided-count",
        suggestion_count = suggestions.len(),
        "Providing suggestions"
    );
    Ok(HttpResponse::Ok().json(SuggestResponse { suggestions }))
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
                    const NUM_PROVIDERS: usize = 2;
                    let mut providers: Vec<Box<dyn SuggestionProvider + Send + Sync>> =
                        Vec::with_capacity(NUM_PROVIDERS);
                    if settings.providers.wiki_fruit.enabled {
                        providers.push(Box::new(WikiFruit));
                    }
                    if settings.providers.adm_rs.enabled {
                        providers.push(Box::new(
                            merino_adm::remote_settings::RemoteSettingsSuggester::default(),
                        ));
                    }

                    let mut multi = merino_suggest::Multi::new(providers);
                    multi.setup(settings).await?;
                    Ok(multi)
                }
                .instrument(setup_span)
            })
            .await
    }
}
