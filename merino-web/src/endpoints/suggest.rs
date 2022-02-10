//! Web handlers for the suggestions API.

use std::collections::HashSet;

use crate::{
    errors::HandlerError, extractors::SuggestionRequestWrapper, providers::SuggestionProviderRef,
};
use actix_web::{
    get,
    web::{self, Data, ServiceConfig},
    HttpMessage, HttpRequest, HttpResponse,
};
use anyhow::Result;
use cadence::{CountedExt, Histogrammed, StatsdClient};
use merino_settings::Settings;
use merino_suggest_traits::{Suggestion, SuggestionProvider, SuggestionRequest};
use serde::{Deserialize, Serialize};
use serde_with::{rust::StringWithSeparator, serde_as, CommaSeparator};
use tracing_actix_web::RequestId;
use uuid::Uuid;

/// Configure a route to use the Suggest service.
pub fn configure(config: &mut ServiceConfig) {
    config.service(suggest);
}

/// Suggest content in response to the queried text.
#[get("")]
#[tracing::instrument(skip(
    metrics_client,
    suggestion_request,
    provider,
    request,
    settings,
    query_parameters
))]
async fn suggest(
    SuggestionRequestWrapper(suggestion_request): SuggestionRequestWrapper,
    provider: Data<SuggestionProviderRef>,
    metrics_client: Data<StatsdClient>,
    query_parameters: web::Query<SuggestQueryParameters>,
    settings: Data<Settings>,
    request: HttpRequest,
) -> Result<HttpResponse, HandlerError> {
    safe_log_request(
        settings.log_full_request,
        &suggestion_request,
        &query_parameters,
    );

    let extensions = request.extensions();
    let request_id: &Uuid = extensions
        .get::<RequestId>()
        .ok_or_else(HandlerError::internal)?;
    let id_multi = &provider.provider.read().await;
    let response = match &query_parameters.providers {
        Some(provider_ids) => {
            id_multi
                .suggest_from_ids(suggestion_request, provider_ids)
                .await
        }
        None => id_multi.suggest(suggestion_request).await,
    }
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
            request_id: *request_id,
        });

    Ok(res)
}

/// Query parameters
#[serde_as]
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct SuggestQueryParameters {
    #[serde_as(as = "StringWithSeparator::<CommaSeparator, String>")]
    /// Query parameter for client_variants
    client_variants: Vec<String>,
    /// Providers to use for this request
    #[serde_as(as = "Option<StringWithSeparator::<CommaSeparator, String>>")]
    providers: Option<HashSet<String>>,
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
    /// A request
    request_id: Uuid,
}

/// Customizes the output format of [`Suggestion`].
#[derive(Debug)]
struct SuggestionWrapper<'a>(&'a Suggestion);

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

        let generated = Generated {
            block_id: self.0.id,
            full_keyword: &self.0.full_keyword,
            title: &self.0.title,
            url: self.0.url.to_string(),
            impression_url: self.0.impression_url.to_string(),
            click_url: self.0.click_url.to_string(),
            provider: &self.0.provider,
            is_sponsored: self.0.is_sponsored,
            icon: self.0.icon.to_string(),
            advertiser: &self.0.advertiser,
            score: self.0.score.into(),
        };

        generated.serialize(serializer)
    }
}

/// Log a suggestion request, respecting the log_query setting passed, and
/// formatting all fields in a way that is helpful to our downstream log
/// handlers. This primarily means that fields don't generate internal
/// stringified JSON.
fn safe_log_request(
    log_query: bool,
    request: &SuggestionRequest,
    query_params: &SuggestQueryParameters,
) {
    let query = if log_query {
        request.query.as_str()
    } else {
        ""
    };
    let requested_providers = query_params.providers.as_ref().map_or_else(
        || "".to_string(),
        |ps| {
            ps.iter()
                .map(|p| p.as_str())
                .collect::<Vec<_>>()
                .as_slice()
                .join(",")
        },
    );

    tracing::info!(
        r#type = "web.suggest.request",
        sensitive = true,
        accepts_english = ?request.accepts_english,
        city = request.city.as_deref(),
        country = request.country.as_deref(),
        os_family = %request.device_info.os_family,
        form_factor = %request.device_info.form_factor,
        browser = %request.device_info.browser,
        dma = request.dma.map(|v| v.to_string()).as_deref(),
        region = request.region.as_deref(),
        %query,
        client_variants = %query_params.client_variants.join(","),
        %requested_providers,
        // Also includes fields from tracing-actix-web-mozlog, including `rid`
        // (request ID), `useragent` and `path` (which does not include query
        // string).
        "handling suggestion request"
    );
}
