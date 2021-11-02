//! Provides a provider-combinator that provides suggestions from multiple sub-providers.

use crate::{
    CacheInputs, CacheStatus, SuggestError, SuggestionProvider, SuggestionRequest,
    SuggestionResponse,
};
use async_trait::async_trait;
use futures::future::join_all;

/// A provider that aggregates suggestions from multiple suggesters.
pub struct Multi {
    /// The providers to aggregate from.
    providers: Vec<Box<dyn SuggestionProvider>>,
}

impl Multi {
    /// Create a `Multi` that draws suggestions from `providers`.
    pub fn new(providers: Vec<Box<dyn SuggestionProvider>>) -> Self {
        Self { providers }
    }

    /// Create a boxed multi
    pub fn new_boxed(providers: Vec<Box<dyn SuggestionProvider>>) -> Box<Self> {
        Box::new(Self::new(providers))
    }
}

#[async_trait]
impl SuggestionProvider for Multi {
    fn name(&self) -> String {
        let provider_names = self
            .providers
            .iter()
            .map(|p| p.name())
            .collect::<Vec<_>>()
            .join(", ");
        format!("Multi({})", provider_names)
    }

    fn cache_inputs(&self, req: &SuggestionRequest, cache_inputs: &mut Box<dyn CacheInputs>) {
        for provider in &self.providers {
            provider.cache_inputs(req, cache_inputs);
        }
    }

    async fn suggest(
        &self,
        request: SuggestionRequest,
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
