//! A suggestion provider that demonstrates the live querying feature
//!
//! It is not meant to be used in production.

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use http::Uri;
use merino_settings::{providers::LiveQueryDemoConfig, Settings};
use serde::Deserialize;

use std::time::Duration;

use merino_suggest_traits::{
    CacheInputs, MakeFreshType, Proportion, SetupError, SuggestError, Suggestion,
    SuggestionProvider, SuggestionRequest, SuggestionResponse,
};

/// User-Agent sent to the suggestion provider.
const REQWEST_USER_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "/", env!("CARGO_PKG_VERSION"),);

/// A suggester that queries against an external suggestion provider.
pub struct LiveQuerySuggester {
    /// The HTTP client to query against the external provider.
    pub client: reqwest::Client,
    /// The configuration of the external provider.
    config: LiveQueryDemoConfig,
}

#[derive(Debug, Deserialize)]
struct DemoSuggestion {
    id: u32,
    url: String,
    image_url: String,
    title: String,
}

#[derive(Debug, Deserialize)]
struct DemoResponse {
    #[allow(dead_code)]
    original_query: String,
    suggestions: Vec<DemoSuggestion>,
}

impl From<DemoSuggestion> for Suggestion {
    fn from(source: DemoSuggestion) -> Suggestion {
        Suggestion {
            provider: "LiveQuerySuggester".to_owned(),
            advertiser: "Wikipedia".to_string(),
            score: Proportion::one(),
            id: source.id,
            full_keyword: source.title.clone(),
            title: source.title.clone(),
            url: source.url.as_str().parse().unwrap(),
            impression_url: Uri::from_static("https://merino.services.mozilla.com/test/impression"),
            click_url: Uri::from_static("https://merino.services.mozilla.com/test/click"),
            is_sponsored: false,
            icon: source.image_url.as_str().parse().unwrap(),
        }
    }
}

impl LiveQuerySuggester {
    /// Create a LiveQuerySuggester from settings.
    pub fn new_boxed(
        settings: Settings,
        config: LiveQueryDemoConfig,
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
                "LiveQuerySuggester can only be used in debug mode",
            )))
        } else {
            Ok(Box::new(Self { client, config }))
        }
    }
}

#[async_trait]
impl SuggestionProvider for LiveQuerySuggester {
    fn name(&self) -> String {
        "LiveQuerySuggester".to_owned()
    }

    fn cache_inputs(&self, req: &SuggestionRequest, cache_inputs: &mut dyn CacheInputs) {
        cache_inputs.add(&[req.accepts_english as u8]);
        cache_inputs.add(req.query.as_bytes());
    }

    async fn suggest(
        &self,
        request: SuggestionRequest,
    ) -> Result<SuggestionResponse, SuggestError> {
        let url = format!("{}?q={}", self.config.endpoint, request.query);
        let response: DemoResponse = self
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
        let res = response.suggestions.into_iter().map(|s| s.into()).collect();
        Ok(SuggestionResponse::new(res))
    }

    async fn reconfigure(
        &mut self,
        new_config: serde_json::Value,
        _make_fresh: &MakeFreshType,
    ) -> Result<(), SetupError> {
        let new_config: LiveQueryDemoConfig = serde_json::from_value(new_config)
            .context("loading provider config")
            .map_err(SetupError::InvalidConfiguration)?;
        self.config = new_config;
        Ok(())
    }
}
