//! API endpoints to introspect the available providers.

use std::collections::HashMap;

use crate::{errors::HandlerError, providers::SuggestionProviderRef};
use actix_web::{
    get, post,
    web::{self, Data, Json},
    HttpResponse,
};
use merino_settings::{Settings, SuggestionProviderConfig};
use merino_suggest_providers::IdMultiProviderDetails;
use serde::Serialize;

/// Configure a route to provide details about the available providers.
pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(list_providers)
        .service(reconfigure_providers);
}

/// Return details about the available providers.
#[get("")]
async fn list_providers(
    provider: Data<SuggestionProviderRef>,
) -> Result<HttpResponse, HandlerError> {
    let id_provider = &provider.provider.read().await;
    let providers = id_provider
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

/// Reconfigures providers on the fly from *unvalidated* user input. *Don't use
/// this in production!*
#[post("reconfigure")]
async fn reconfigure_providers(
    config: Json<HashMap<String, SuggestionProviderConfig>>,
    provider: Data<SuggestionProviderRef>,
    settings: Data<Settings>,
) -> Result<HttpResponse, HandlerError> {
    if settings.debug {
        provider
            .reconfigure(config.clone())
            .await
            .map_err(|_| HandlerError::internal())?;
        Ok(HttpResponse::NoContent().body(""))
    } else {
        Ok(HttpResponse::NotFound().body(""))
    }
}
