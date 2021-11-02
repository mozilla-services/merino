//! A suggestion provider that provides toy responses.
//!
//! It is useful in that it is fully self contained and very simple. It is meant
//! to be used in development and testing.

use std::marker::PhantomData;

use anyhow::anyhow;
use async_trait::async_trait;
use http::Uri;
use merino_settings::Settings;

use crate::{
    domain::Proportion, CacheInputs, SetupError, SuggestError, Suggestion, SuggestionProvider,
    SuggestionRequest, SuggestionResponse,
};

/// A toy suggester to test the system.
pub struct WikiFruit {
    /// A zero-sized private field to ensure that the type cannot be directly created.
    _phantom: PhantomData<()>,
}

impl WikiFruit {
    /// Create a WikiFruit provider from settings.
    pub fn new_boxed(settings: &Settings) -> Result<Box<Self>, SetupError> {
        if !settings.debug {
            Err(SetupError::InvalidConfiguration(anyhow!(
                "WikiFruit suggestion provider can only be used in debug mode",
            )))
        } else {
            Ok(Box::new(Self {
                _phantom: PhantomData,
            }))
        }
    }
}

#[async_trait]
impl SuggestionProvider for WikiFruit {
    fn name(&self) -> String {
        "WikiFruit".to_string()
    }

    fn cache_inputs(&self, req: &SuggestionRequest, cache_inputs: &mut Box<dyn CacheInputs>) {
        cache_inputs.add(req.query.as_bytes());
    }

    async fn suggest(
        &self,
        request: SuggestionRequest,
    ) -> Result<SuggestionResponse, SuggestError> {
        let suggestion = match request.query.as_ref() {
            "apple" => Some(Suggestion {
                id: 1,
                full_keyword: "apple".to_string(),
                title: "Wikipedia - Apple".to_string(),
                url: Uri::from_static("https://en.wikipedia.org/wiki/Apple"),
                impression_url: Uri::from_static("https://127.0.0.1/"),
                click_url: Uri::from_static("https://127.0.0.1/"),
                provider: "Merino::WikiFruit".to_string(),
                is_sponsored: false,
                icon: Uri::from_static("https://en.wikipedia.org/favicon.ico"),
                score: Proportion::zero(),
            }),
            "banana" => Some(Suggestion {
                id: 1,
                full_keyword: "banana".to_string(),
                title: "Wikipedia - Banana".to_string(),
                url: Uri::from_static("https://en.wikipedia.org/wiki/Banana"),
                impression_url: Uri::from_static("https://127.0.0.1/"),
                click_url: Uri::from_static("https://127.0.0.1/"),
                provider: "Merino::WikiFruit".to_string(),
                is_sponsored: false,
                icon: Uri::from_static("https://en.wikipedia.org/favicon.ico"),
                score: Proportion::zero(),
            }),
            "cherry" => Some(Suggestion {
                id: 1,
                full_keyword: "cherry".to_string(),
                title: "Wikipedia - Cherry".to_string(),
                url: Uri::from_static("https://en.wikipedia.org/wiki/Cherry"),
                impression_url: Uri::from_static("https://127.0.0.1/"),
                click_url: Uri::from_static("https://127.0.0.1/"),
                provider: "Merino::WikiFruit".to_string(),
                is_sponsored: false,
                icon: Uri::from_static("https://en.wikipedia.org/favicon.ico"),
                score: Proportion::zero(),
            }),
            _ => None,
        };

        Ok(SuggestionResponse::new(suggestion.into_iter().collect()))
    }
}
