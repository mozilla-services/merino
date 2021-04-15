use actix_web::{get, web, HttpResponse};
use serde::{Deserialize, Serialize};

use merino_suggest::{Suggester, Suggestion, WikiFruit};

/// Handles suggesting completions for Quantumbar queries
pub fn service(config: &mut web::ServiceConfig) {
    config.service(suggest);
}

/// Suggest content in response to the queried text
#[get("")]
fn suggest(query: web::Query<SuggestQuery>) -> HttpResponse {
    let suggestions = WikiFruit::suggest(&query.q);
    HttpResponse::Ok().json(SuggestResponse { suggestions })
}

#[derive(Debug, Deserialize)]
struct SuggestQuery {
    q: String,
}

#[derive(Debug, Serialize)]
struct SuggestResponse {
    suggestions: Vec<Suggestion>,
}
