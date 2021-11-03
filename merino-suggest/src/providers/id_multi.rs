//! Provides a provider-combinator that contains a set of named providers. It
//! can list these providers by name, and serve suggestions using only a partial
//! set of them.
//!
//! This is intended to be used only as the top level provider for the service.

use std::collections::{HashMap, HashSet};

use crate::{CacheStatus, SuggestError, SuggestionProvider, SuggestionRequest, SuggestionResponse};
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

    async fn suggest(
        &self,
        request: SuggestionRequest,
    ) -> Result<SuggestionResponse, SuggestError> {
        let ids: HashSet<_> = self.providers.keys().cloned().collect();
        self.suggest_from_ids(request, &ids).await
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        CacheStatus, IdMulti, SuggestError, SuggestionProvider, SuggestionRequest,
        SuggestionResponse,
    };
    use async_trait::async_trait;
    use fake::{Fake, Faker};
    use tokio::sync::oneshot::error::TryRecvError;

    /// A provider that can be externally paused mid-request.
    struct ChannelProvider {
        tx: tokio::sync::mpsc::Sender<()>,
        rx: tokio::sync::Mutex<tokio::sync::mpsc::Receiver<()>>,
    }

    #[async_trait]
    impl SuggestionProvider for ChannelProvider {
        fn name(&self) -> String {
            "channel".to_string()
        }

        async fn suggest(
            &self,
            _request: SuggestionRequest,
        ) -> Result<SuggestionResponse, SuggestError> {
            self.tx.send(()).await.unwrap();
            self.rx.lock().await.recv().await.unwrap();

            Ok(SuggestionResponse {
                cache_status: CacheStatus::NoCache,
                cache_ttl: None,
                suggestions: vec![],
            })
        }
    }

    #[tokio::test]
    async fn multi_is_concurrent() {
        // Set up two way communication for two internal providers, and a one shot provider to get the suggestion out of a thread.
        let (prov1_input_tx, prov1_input_rx) = tokio::sync::mpsc::channel::<()>(4);
        let (prov1_output_tx, mut prov1_output_rx) = tokio::sync::mpsc::channel::<()>(4);
        let (prov2_input_tx, prov2_input_rx) = tokio::sync::mpsc::channel::<()>(4);
        let (prov2_output_tx, mut prov2_output_rx) = tokio::sync::mpsc::channel::<()>(4);
        let (suggestion_result_tx, mut suggestion_result_rx) =
            tokio::sync::oneshot::channel::<()>();

        // Set up the providers
        let mut providers: HashMap<_, Box<dyn SuggestionProvider>> = HashMap::new();
        providers.insert(
            "1".to_string(),
            Box::new(ChannelProvider {
                tx: prov1_output_tx,
                rx: tokio::sync::Mutex::new(prov1_input_rx),
            }),
        );
        providers.insert(
            "2".to_string(),
            Box::new(ChannelProvider {
                tx: prov2_output_tx,
                rx: tokio::sync::Mutex::new(prov2_input_rx),
            }),
        );
        let multi = IdMulti::new(providers);

        // Start a request that will use both prov1 and prov2 via a multi provider.
        let task_handle = tokio::spawn(async move {
            let request: SuggestionRequest = Faker.fake();
            multi.suggest(request).await.unwrap();
            // Signal that the request has finished
            suggestion_result_tx.send(()).unwrap();
        });

        // Confirm that both providers have called and have sent a message over
        // their output channel (requesting to continue). This is the most
        // important assertion, as it demonstrates that both providers have
        // started before either of them have finished.
        tokio::join!(prov1_output_rx.recv(), prov2_output_rx.recv());

        // Make sure no response has been received
        assert!(matches!(
            suggestion_result_rx.try_recv(),
            Err(TryRecvError::Empty)
        ));

        // Allow one provider to continue
        prov1_input_tx.send(()).await.unwrap();

        // Make sure no response has been received
        assert!(matches!(
            suggestion_result_rx.try_recv(),
            Err(TryRecvError::Empty)
        ));

        // Allow the other provider to continue
        prov2_input_tx.send(()).await.unwrap();

        // Wait for the response.
        suggestion_result_rx.await.unwrap();
        task_handle.await.unwrap();
    }
}
