//! Provides a provider-combinator that provides suggestions from multiple sub-providers.

use crate::{SuggestError, Suggestion, SuggestionProvider};
use async_trait::async_trait;
use futures::future::join_all;

/// A provider that aggregates suggestions from multiple suggesters.
pub struct Multi<'a> {
    /// The providers to aggregate from.
    providers: Vec<Box<dyn SuggestionProvider<'a> + Send + Sync>>,
}

impl<'a> Multi<'a> {
    /// Create a `Multi` that draws suggestions from `providers`.
    pub fn new(providers: Vec<Box<dyn SuggestionProvider<'a> + Send + Sync>>) -> Self {
        Self { providers }
    }
}

#[async_trait]
impl<'a> SuggestionProvider<'a> for Multi<'a> {
    fn name(&self) -> std::borrow::Cow<'a, str> {
        let provider_names = self
            .providers
            .iter()
            .map(|p| p.name())
            .collect::<Vec<_>>()
            .join(", ");
        format!("{}({})", "Multi", provider_names).into()
    }

    async fn setup(
        &mut self,
        settings: &merino_settings::Settings,
    ) -> Result<(), crate::SetupError> {
        for provider in &mut self.providers {
            provider.setup(settings).await?;
        }
        Ok(())
    }

    async fn suggest(&self, query: &str) -> Result<Vec<Suggestion>, SuggestError> {
        // collect a Vec<Result<Vec<T>, E>>, and then transpose it into a Result<Vec<Vec<T>>, E>.
        let v: Result<Vec<Vec<_>>, _> = join_all(self.providers.iter().map(|p| p.suggest(query)))
            .await
            .into_iter()
            .collect();
        // now flatten it
        let suggestions = v?.into_iter().flatten().collect();
        Ok(suggestions)
    }
}
