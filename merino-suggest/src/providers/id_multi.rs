//! Provides a provider-combinator that contains a set of named providers. It
//! can list these providers by name, and serve suggestions using only a partial
//! set of them.
//!
//! This is intended to be used only as the top level provider for the service.

use std::collections::{HashMap, HashSet};

use crate::{
    CacheInputs, CacheStatus, SuggestError, SuggestionProvider, SuggestionRequest,
    SuggestionResponse,
};
use async_trait::async_trait;
use futures::{future::join_all, TryFutureExt};
use serde::Serialize;

/// A provider that aggregates suggestions from suggesters that tracks an ID per
/// suggester (or suggester tree).
#[derive(Default)]
pub struct IdMulti {
    /// The providers to aggregate from.
    providers: HashMap<String, Box<dyn SuggestionProvider>>,
}

/// Metadata about a provider contained in [`NamedMulti`];
#[derive(Debug, Serialize)]
pub struct ProviderDetails {
    /// The id of this provider. This is presented to the users in the API.
    pub id: String,
    /// The availability of this provider, which affects if it is used by
    /// default in requests, and how clients can configure it.
    pub availability: ProviderAvailability,
}

/// The availability of a provider, which affects if it is used by
/// default in requests, and how clients can configure it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderAvailability {
    /// The provider is enabled by default, can be disabled by clients, and
    /// should be shown in user configuration interfaces.
    EnabledByDefault,
}

impl IdMulti {
    /// Create a `Multi` that draws suggestions from `providers`.
    #[must_use]
    pub fn new(providers: HashMap<String, Box<dyn SuggestionProvider>>) -> Self {
        Self { providers }
    }

    /// Modify this provider to include another named provider tree.
    pub fn add_provider(&mut self, name: &str, provider: Box<dyn SuggestionProvider>) -> &mut Self {
        if !provider.is_null() {
            self.providers.insert(name.to_string(), provider);
        }
        self
    }

    /// Return metadata about the contained providers.
    #[must_use]
    pub fn list_providers(&self) -> Vec<ProviderDetails> {
        self.providers
            .keys()
            .map(|id| ProviderDetails {
                id: id.clone(),
                availability: ProviderAvailability::EnabledByDefault,
            })
            .collect()
    }

    /// Provide suggested results for `query` using only the providers listed by ID.
    ///
    /// # Errors
    /// Returns an error if any sub providers return an error.
    pub async fn suggest_from_ids(
        &self,
        request: SuggestionRequest,
        ids: &HashSet<String>,
    ) -> Result<SuggestionResponse, SuggestError> {
        // make a Vec<Result<Vec<T>, E>>...
        let v: Result<Vec<SuggestionResponse>, _> = join_all(
            self.providers
                .iter()
                .filter(|(name, _)| ids.contains(*name))
                .map(|(name, provider)| {
                    // Change the provider name to the name of the group specified in the config.
                    let name = name.clone();
                    provider.suggest(request.clone()).map_ok(move |mut res| {
                        res.suggestions
                            .iter_mut()
                            .for_each(move |s| s.provider = name.clone());
                        res
                    })
                }),
        )
        .await
        .into_iter()
        // ...and then transpose it into a Result<Vec<Vec<T>>, E>.
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

#[async_trait]
impl SuggestionProvider for IdMulti {
    fn name(&self) -> String {
        let provider_names = self
            .providers
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        format!("NamedMulti({})", provider_names)
    }

    fn cache_inputs(&self, req: &SuggestionRequest, cache_inputs: &mut dyn CacheInputs) {
        for provider in self.providers.values() {
            provider.cache_inputs(req, cache_inputs);
        }
    }

    async fn suggest(
        &self,
        request: SuggestionRequest,
    ) -> Result<SuggestionResponse, SuggestError> {
        let ids: HashSet<_> = self.providers.keys().cloned().collect();
        self.suggest_from_ids(request, &ids).await
    }
}
