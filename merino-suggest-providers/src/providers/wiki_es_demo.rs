//! A suggestion provider that queries the Wikipedia Elasticsearch backend.
//!
//! It is not meant to be used in production.

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use elasticsearch::SearchParts;
use http::Uri;
use merino_settings::{providers::WikiEsDemoConfig, Settings};
use serde::Deserialize;
use serde_json::{json, Value};

use merino_suggest_traits::{
    CacheInputs, MakeFreshType, Proportion, SetupError, SuggestError, Suggestion,
    SuggestionProvider, SuggestionRequest, SuggestionResponse,
};
use merino_wikipedia::ElasticHelper;

const IMAGE_URL: &'static str = "https://firefox-settings-attachments.cdn.mozilla.net/main-workspace/quicksuggest/56691f56-802f-4174-95a7-d899683faaf5";

/// A suggester that queries against the Elasticsearch provider.
pub struct WikiEsSuggester {
    /// The Elasticsearch helper.
    pub es_helper: ElasticHelper,
    /// The configuration of the provider.
    config: WikiEsDemoConfig,
}

#[derive(Debug, Deserialize)]
struct APIResponse(String, Vec<String>, Vec<String>, Vec<String>);

impl WikiEsSuggester {
    /// Create a WikiEsSuggester from settings.
    pub fn new_boxed(
        settings: Settings,
        config: WikiEsDemoConfig,
    ) -> Result<Box<Self>, SetupError> {
        let es_helper = ElasticHelper::new(&settings.elasticsearch, config.index.clone())
            .map_err(|e| SetupError::Network(e.into()))?;
        if !settings.debug {
            Err(SetupError::InvalidConfiguration(anyhow!(
                "WikiEsSuggester can only be used in debug mode",
            )))
        } else {
            Ok(Box::new(Self { es_helper, config }))
        }
    }
}

#[async_trait]
impl SuggestionProvider for WikiEsSuggester {
    fn name(&self) -> String {
        "WikiEsSuggester".to_owned()
    }

    fn cache_inputs(&self, req: &SuggestionRequest, cache_inputs: &mut dyn CacheInputs) {
        cache_inputs.add(&[req.accepts_english as u8]);
        cache_inputs.add(req.query.as_bytes());
    }

    async fn suggest(
        &self,
        request: SuggestionRequest,
    ) -> Result<SuggestionResponse, SuggestError> {
        let response = self
            .es_helper
            .client
            .search(SearchParts::Index(&[&self.es_helper.index_name]))
            .from(0)
            .size(5)
            .body(json!({
                "query": {
                    "multi_match": {
                        "query": request.query,
                        "type": "bool_prefix",
                        "fields": ["title", "title._2gram", "title._3gram"]
                    }
                }
            }))
            .send()
            .await
            .map_err(|e| SuggestError::Network(anyhow!("Couldn't search from the backend: {}", e)))?
            .json::<Value>()
            .await
            .map_err(|e| {
                SuggestError::Internal(anyhow!("Failed to parse the JSON response: {}", e))
            })?;

        let res = response["hits"]["hits"]
            .as_array()
            .expect("Hits")
            .iter()
            .map(|article| Suggestion {
                provider: "Elasticsearch".to_owned(),
                advertiser: "Wikipedia".to_string(),
                score: Proportion::one(),
                id: 0,
                full_keyword: article["_source"]["title"]
                    .as_str()
                    .expect("Title")
                    .to_owned(),
                title: article["_source"]["title"]
                    .as_str()
                    .expect("Title")
                    .to_owned(),
                url: article["_source"]["url"]
                    .as_str()
                    .expect("Url")
                    .parse()
                    .unwrap(),
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
        let new_config: WikiEsDemoConfig = serde_json::from_value(new_config)
            .context("loading provider config")
            .map_err(SetupError::InvalidConfiguration)?;
        self.config = new_config;
        Ok(())
    }
}
