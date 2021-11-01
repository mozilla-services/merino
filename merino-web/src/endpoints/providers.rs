//! API endpoints to introspect the available providers.

use std::collections::HashMap;

use crate::{errors::HandlerError, providers::SuggestionProviderRef};
use actix_web::{
    get,
    web::{self, Data},
    HttpResponse,
};
use merino_suggest::IdMultiProviderDetails;
use serde::Serialize;

/// Configure a route to provide details about the available providers.
pub fn configure(config: &mut web::ServiceConfig) {
    config.service(list_providers);
}

/// Return details about the available providers.
#[get("")]
async fn list_providers(
    provider: Data<SuggestionProviderRef>,
) -> Result<HttpResponse, HandlerError> {
    let provider = provider.0.clone();
    let providers = provider
        .list_providers()
        .into_iter()
        .map(|p| (p.id.clone(), p))
        .collect();

    Ok(HttpResponse::Ok()
        .append_header(("Cache-Control", "public, max-age=900".to_string()))
        .json(ListResponse { providers }))
}

/// The response given in the API
#[derive(Debug, Serialize)]
struct ListResponse {
    /// details about the providers
    providers: HashMap<String, IdMultiProviderDetails>,
}
