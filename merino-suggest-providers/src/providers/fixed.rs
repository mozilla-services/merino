//! A suggestion provider that provides a fixed response with a customizable
//! title.
//!
//! It is meant to be used in development and testing.

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use http::Uri;
use merino_settings::{providers::FixedConfig, Settings};

use merino_suggest_traits::{
    CacheInputs, MakeFreshType, Proportion, SetupError, SuggestError, Suggestion,
    SuggestionProvider, SuggestionRequest, SuggestionResponse,
};

/// A suggester that always provides the same suggestion, with a configurable title.
pub struct FixedProvider {
    /// The string that will be returned in the title.
    pub value: String,
}

impl FixedProvider {
    /// Create a DebugProvider provider from settings.
    ///
    /// The `provider` field of the suggestion will be overwritten.
    pub fn new_boxed(settings: Settings, config: FixedConfig) -> Result<Box<Self>, SetupError> {
        if !settings.debug {
            Err(SetupError::InvalidConfiguration(anyhow!(
                "FixedProvider can only be used in debug mode",
            )))
        } else {
            Ok(Box::new(Self {
                value: config.value,
            }))
        }
    }
}

#[async_trait]
impl SuggestionProvider for FixedProvider {
    fn name(&self) -> String {
        format!("FixedProvider({})", self.value)
    }

    fn cache_inputs(&self, _req: &SuggestionRequest, _cache_inputs: &mut dyn CacheInputs) {
        // No property of req will change the response
    }

    async fn suggest(
        &self,
        _request: SuggestionRequest,
    ) -> Result<SuggestionResponse, SuggestError> {
        Ok(SuggestionResponse::new(vec![Suggestion {
            provider: self.name(),
            advertiser: "test_advertiser".to_string(),
            score: Proportion::zero(),
            id: 0,
            full_keyword: "".to_string(),
            title: self.value.clone(),
            url: Uri::from_static("https://merino.services.mozilla.com/test/suggestion"),
            impression_url: Uri::from_static("https://merino.services.mozilla.com/test/impression"),
            click_url: Uri::from_static("https://merino.services.mozilla.com/test/click"),
            is_sponsored: false,
            icon: Uri::from_static("https://mozilla.com/favicon.png"),
        }]))
    }

    async fn reconfigure(
        &mut self,
        new_config: serde_json::Value,
        _make_fresh: &MakeFreshType,
    ) -> Result<(), SetupError> {
        let new_config: FixedConfig = serde_json::from_value(new_config)
            .context("loading provider config")
            .map_err(SetupError::InvalidConfiguration)?;
        self.value = new_config.value;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::FixedProvider;
    use merino_settings::providers::{FixedConfig, SuggestionProviderConfig};
    use merino_suggest_traits::{MakeFreshType, SuggestionProvider};

    #[tokio::test]
    async fn test_reconfigure() {
        let mut provider = FixedProvider {
            value: "foo".to_owned(),
        };

        // This won't be called as `DelayProvider::reconfigure()` will always succeed.
        let make_fresh: MakeFreshType = Box::new(move |_fresh_config: SuggestionProviderConfig| {
            unreachable!();
        });

        let value = serde_json::to_value(FixedConfig {
            value: "bar".to_owned(),
        })
        .expect("failed to serialize");
        provider
            .reconfigure(value, &make_fresh)
            .await
            .expect("failed to reconfigure");
        assert_eq!(provider.value, "bar");
    }
}
