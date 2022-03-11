//! A suggestion provider that queries the Wikipedia OpenSearch API.
//!
//! It is not meant to be used in production.

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use http::Uri;
use merino_settings::{providers::WikipediaOpenSearchConfig, Settings};
use serde::Deserialize;

use std::time::Duration;

use merino_suggest_traits::{
    CacheInputs, MakeFreshType, Proportion, SetupError, SuggestError, Suggestion,
    SuggestionProvider, SuggestionRequest, SuggestionResponse,
};

/// User-Agent sent to the suggestion provider.
const REQWEST_USER_AGENT: &'static str =
    "Mozilla/5.0 (Windows NT 10.0; rv:10.0) Gecko/20100101 Firefox/98.0";
const IMAGE_URL: &'static str = "https://firefox-settings-attachments.cdn.mozilla.net/main-workspace/quicksuggest/56691f56-802f-4174-95a7-d899683faaf5";

/// A suggester that queries against an external suggestion provider.
pub struct WikiOpenSearchSuggester {
    /// The HTTP client to query against the external provider.
    pub client: reqwest::Client,
    /// The configuration of the external provider.
    config: WikipediaOpenSearchConfig,
}

#[derive(Debug, Deserialize)]
struct APIResponse(String, Vec<String>, Vec<String>, Vec<String>);

impl WikiOpenSearchSuggester {
    /// Create a WikiOpenSearchSuggester from settings.
    pub fn new_boxed(
        settings: Settings,
        config: WikipediaOpenSearchConfig,
    ) -> Result<Box<Self>, SetupError> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(1))
            .user_agent(REQWEST_USER_AGENT)
            .build()
            .context("Unable to create the Reqwest client")
            .map_err(SetupError::Network)?;
        if !settings.debug {
            Err(SetupError::InvalidConfiguration(anyhow!(
                "WikiOpenSearchSuggester can only be used in debug mode",
            )))
        } else {
            Ok(Box::new(Self { client, config }))
        }
    }
}

#[async_trait]
impl SuggestionProvider for WikiOpenSearchSuggester {
    fn name(&self) -> String {
        "WikiOpenSearchSuggester".to_owned()
    }

    fn cache_inputs(&self, req: &SuggestionRequest, cache_inputs: &mut dyn CacheInputs) {
        cache_inputs.add(&[req.accepts_english as u8]);
        cache_inputs.add(req.query.as_bytes());
    }

    async fn suggest(
        &self,
        request: SuggestionRequest,
    ) -> Result<SuggestionResponse, SuggestError> {
        let url = format!("{}&search={}", self.config.endpoint, request.query);
        tracing::info!(r#type="merino.OpenSearch", ?url, "URL:");
        let response: APIResponse = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|_e| SuggestError::Network(anyhow!("Couldn't reach to provider")))?
            .error_for_status()
            .map_err(|e| SuggestError::Network(e.into()))?
            .json()
            .await
            .map_err(|e| {
                SuggestError::Internal(anyhow!("Failed to parse the JSON response: {}", e))
            })?;

        let res = response
            .1
            .iter()
            .zip(response.3.iter())
            .map(|(title, link)| Suggestion {
                provider: "OpenSearch".to_owned(),
                advertiser: "Wikipedia".to_string(),
                score: Proportion::one(),
                id: 0,
                full_keyword: title.clone(),
                title: title.clone(),
                url: link.parse().unwrap(),
                impression_url: Uri::from_static(
                    "https://merino.services.mozilla.com/test/impression",
                ),
                click_url: Uri::from_static("https://merino.services.mozilla.com/test/click"),
                is_sponsored: false,
                icon: IMAGE_URL.parse().unwrap(),
            })
            .collect();
        Ok(SuggestionResponse::new(res))
    }

    async fn reconfigure(
        &mut self,
        new_config: serde_json::Value,
        _make_fresh: &MakeFreshType,
    ) -> Result<(), SetupError> {
        let new_config: WikipediaOpenSearchConfig = serde_json::from_value(new_config)
            .context("loading provider config")
            .map_err(SetupError::InvalidConfiguration)?;
        self.config = new_config;
        Ok(())
    }
}
