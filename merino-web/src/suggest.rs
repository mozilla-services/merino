//! Web handlers for the suggestions API.

use actix_web::{
    get,
    web::{Data, Query, ServiceConfig},
    HttpResponse,
};
use anyhow::{Context, Result};
use merino_settings::Settings;
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;

use merino_suggest::{Suggestion, SuggestionProvider, WikiFruit};
use tracing::instrument;
use tracing_futures::WithSubscriber;

use crate::errors::HandlerError;

/// A set of suggesters stored in Actix's app_data.
type SuggesterSet<'a> = OnceCell<Vec<Box<dyn SuggestionProvider<'a>>>>;

/// Configure a route to use the Suggest service.
pub fn configure(config: &mut ServiceConfig) {
    config
        .data::<SuggesterSet>(OnceCell::new())
        .service(suggest);
}

/// Set up configured suggestion providers.
#[instrument("suggester-setup", skip(settings))]
async fn setup_suggesters<'a>(settings: &Settings) -> Result<Vec<Box<dyn SuggestionProvider<'a>>>> {
    tracing::info!(
        r#type = "web.configuring-suggesters",
        "Setting up suggestion providers"
    );

    /// The most providers we expect to have at once.
    const MAX_PROVIDERS: usize = 2;
    let mut providers: Vec<Box<dyn SuggestionProvider>> = Vec::with_capacity(MAX_PROVIDERS);

    if settings.providers.wiki_fruit.enabled {
        providers.push(Box::new(WikiFruit));
    }

    if settings.providers.adm_rs.enabled {
        providers.push(Box::new(
            merino_adm::remote_settings::RemoteSettingsSuggester::default(),
        ));
    }

    for provider in &mut providers {
        provider
            .setup(settings)
            .await
            .context(format!("Setting up provider {}", provider.name()))?;
    }

    Ok(providers)
}

/// Suggest content in response to the queried text.
#[get("")]
#[tracing::instrument(skip(query, suggesters, settings))]
async fn suggest<'a>(
    query: Query<SuggestQuery>,
    suggesters: Data<SuggesterSet<'a>>,
    settings: Data<Settings>,
) -> Result<HttpResponse, HandlerError> {
    let suggesters = suggesters
        .get_or_try_init(|| setup_suggesters(settings.as_ref()))
        .with_current_subscriber()
        .await
        .map_err(|error| {
            tracing::error!(
                ?error,
                r#type = "web.suggest.setup-error",
                "suggester error"
            );
            HandlerError::Internal
        })?;

    let mut suggestions: Vec<Suggestion> = Vec::new();
    for provider in suggesters {
        match provider.suggest(&query.q).await {
            Ok(provider_suggestions) => {
                suggestions.extend_from_slice(&provider_suggestions);
            }
            Err(error) => {
                tracing::error!(%error, "Error providing suggestions");
            }
        }
    }
    tracing::debug!(
        r#type = "web.suggest.provided-count",
        suggestion_count = suggestions.len(),
        "Providing suggestions"
    );
    Ok(HttpResponse::Ok().json(SuggestResponse { suggestions }))
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
