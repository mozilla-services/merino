//! Web handlers for the suggestions API.

use actix_web::{get, web, HttpResponse};
use serde::{Deserialize, Serialize};

use merino_suggest::{Suggester, Suggestion, WikiFruit};

/// Handles suggesting completions for Quantumbar queries.
pub fn service(config: &mut web::ServiceConfig) {
    config.service(suggest);
}

/// Suggest content in response to the queried text.
#[get("")]
fn suggest(query: web::Query<SuggestQuery>) -> HttpResponse {
    let suggestions = WikiFruit.suggest(&query.q);
    HttpResponse::Ok().json(SuggestResponse { suggestions })
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
