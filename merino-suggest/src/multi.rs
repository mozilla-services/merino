//! Provides a provider-combinator that provides suggestions from multiple sub-providers.

use crate::{CacheStatus, SuggestError, SuggestionProvider, SuggestionRequest, SuggestionResponse};
use async_trait::async_trait;
use futures::future::join_all;

/// Type alias for the contained suggestion type to save some typing.
type ThreadSafeSuggestionProvider<'a> = Box<dyn SuggestionProvider<'a> + Send + Sync>;

/// A provider that aggregates suggestions from multiple suggesters.
pub struct Multi<'a> {
    /// The providers to aggregate from.
    providers: Vec<ThreadSafeSuggestionProvider<'a>>,
}

impl<'a> Multi<'a> {
    /// Create a `Multi` that draws suggestions from `providers`.
    pub fn new(providers: Vec<ThreadSafeSuggestionProvider<'a>>) -> Self {
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
        join_all(
            self.providers
                .iter_mut()
                .map(|provider| provider.setup(settings)),
        )
        .await
        // Vec<Result<T, E>> -> Result<(), E>. `Ok` if all providers set up
        // correctly. If any failed, returns the first error.
        .into_iter()
        .collect::<Result<(), _>>()
    }

    async fn suggest(
        &self,
        request: SuggestionRequest<'a>,
    ) -> Result<SuggestionResponse, SuggestError> {
        // collect a Vec<Result<Vec<T>, E>>, and then transpose it into a Result<Vec<Vec<T>>, E>.
        let v: Result<Vec<SuggestionResponse>, _> =
            join_all(self.providers.iter().map(|p| p.suggest(request.clone())))
                .await
                .into_iter()
                .collect();
        // now flatten it
        v.map(|mut responses| {
            let mut rv = responses
                .pop()
                .unwrap_or_else(|| SuggestionResponse::new(vec![]));

            for response in responses {
                rv.suggestions.extend_from_slice(&response.suggestions);
                rv.cache_status = match (rv.cache_status, response.cache_status) {
                    (a, b) if a == b => a,
                    (a, CacheStatus::NoCache) => a,
                    _ => CacheStatus::Mixed,
                }
            }

            rv
        })
    }
}
