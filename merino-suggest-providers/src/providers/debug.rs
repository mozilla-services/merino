//! A suggestion provider that provides debug responses.
//!
//! It is meant to be used in development and testing.

use std::marker::PhantomData;

use anyhow::anyhow;
use async_trait::async_trait;
use fake::{Fake, Faker};
use merino_settings::Settings;

use merino_suggest_traits::{
    Proportion, SetupError, SuggestError, Suggestion, SuggestionProvider, SuggestionRequest,
    SuggestionResponse,
};

/// A toy suggester to test the system.
pub struct DebugProvider {
    /// A zero-sized private field to ensure that the type cannot be directly created.
    _phantom: PhantomData<()>,
}

impl DebugProvider {
    /// Create a DebugProvider provider from settings.
    pub fn new_boxed(settings: Settings) -> Result<Box<Self>, SetupError> {
        if !settings.debug {
            Err(SetupError::InvalidConfiguration(anyhow!(
                "DebugProvider can only be used in debug mode",
            )))
        } else {
            Ok(Box::new(Self {
                _phantom: PhantomData,
            }))
        }
    }
}

#[async_trait]
impl SuggestionProvider for DebugProvider {
    fn name(&self) -> String {
        "DebugProvider".to_string()
    }

    async fn suggest(
        &self,
        request: SuggestionRequest,
    ) -> Result<SuggestionResponse, SuggestError> {
        let json: String = serde_json::to_string(&request).map_err(SuggestError::Serialization)?;

        Ok(SuggestionResponse::new(vec![Suggestion {
            title: json,
            provider: "Merino::Debug".into(),
            score: Proportion::zero(),
            ..Faker.fake()
        }]))
    }
}
