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

use merino_suggest::{Suggester, Suggestion, WikiFruit};

use crate::errors::HandlerError;

/// A set of suggesters stored in Actix's app_data.
type SuggesterSet = OnceCell<Vec<Box<dyn Suggester>>>;

/// Configure a route to use the Suggest service.
pub fn configure(config: &mut ServiceConfig) {
    config
        .data::<SuggesterSet>(OnceCell::new())
        .service(suggest);
}

/// Set up configured suggestion providers.
async fn setup_suggesters(settings: &Settings) -> Result<Vec<Box<dyn Suggester>>> {
    println!("Setting up suggesters");
    let mut adm_rs_provider = merino_adm::remote_settings::RemoteSettingsSuggester::default();
    adm_rs_provider
        .sync(settings)
        .await
        .context("syncing provider")?;
    Ok(vec![Box::new(WikiFruit), Box::new(adm_rs_provider)])
}

/// Suggest content in response to the queried text.
#[get("")]
async fn suggest(
    query: Query<SuggestQuery>,
    suggesters: Data<SuggesterSet>,
    settings: Data<&Settings>,
) -> Result<HttpResponse, HandlerError> {
    let suggesters = suggesters
        .get_or_try_init(|| setup_suggesters(settings.as_ref()))
        .await
        .map_err(|e| {
            println!(
                "suggester error {:?}\nchain: {:?}",
                e,
                e.chain().collect::<Vec<_>>(),
            );
            HandlerError::Internal
        })?;
    let suggestions = suggesters
        .iter()
        .flat_map(|sug| sug.suggest(&query.q))
        .collect();
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
