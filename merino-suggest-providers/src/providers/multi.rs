//! Provides a provider-combinator that provides suggestions from multiple sub-providers.

use async_trait::async_trait;
use futures::StreamExt;
use futures::{future::join_all, stream::FuturesUnordered};
use merino_settings::providers::MultiplexerConfig;
use merino_suggest_traits::{
    convert_config, reconfigure_or_remake, CacheInputs, CacheStatus, MakeFreshType, SetupError,
    SuggestError, SuggestionProvider, SuggestionRequest, SuggestionResponse,
};

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

    fn cache_inputs(&self, req: &SuggestionRequest, cache_inputs: &mut dyn CacheInputs) {
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

    async fn reconfigure(
        &mut self,
        new_config: serde_json::Value,
        make_fresh: &MakeFreshType,
    ) -> Result<(), SetupError> {
        let new_config: MultiplexerConfig = convert_config(new_config)?;

        if new_config.providers.len() == self.providers.len() {
            // be optimistic. maybe nothing changed too drastically
            for (conf, prov) in new_config
                .providers
                .into_iter()
                .zip(self.providers.iter_mut())
            {
                reconfigure_or_remake(prov, conf, make_fresh).await?;
            }
        } else {
            // Be pessimistic and just throw everything away and recreate it all
            // TODO This can probably be better, right?
            self.providers.truncate(0);
            let mut queue = FuturesUnordered::new();
            for conf in new_config.providers {
                queue.push(make_fresh(conf));
            }
            while let Some(result) = queue.next().await {
                self.providers.push(result?);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Multi;
    use crate::FixedProvider;
    use async_trait::async_trait;
    use fake::{Fake, Faker};
    use futures::{future::ready, FutureExt};
    use merino_settings::providers::{FixedConfig, MultiplexerConfig, SuggestionProviderConfig};
    use merino_suggest_traits::{
        CacheStatus, MakeFreshType, NullProvider, SetupError, SuggestError, SuggestionProvider,
        SuggestionRequest, SuggestionResponse,
    };
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

        async fn reconfigure(
            &mut self,
            _new_config: serde_json::Value,
            _make_fresh: &MakeFreshType,
        ) -> Result<(), SetupError> {
            unimplemented!()
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
        let prov1 = ChannelProvider {
            tx: prov1_output_tx,
            rx: tokio::sync::Mutex::new(prov1_input_rx),
        };
        let prov2 = ChannelProvider {
            tx: prov2_output_tx,
            rx: tokio::sync::Mutex::new(prov2_input_rx),
        };
        let multi = Multi::new(vec![Box::new(prov1), Box::new(prov2)]);

        // Start a request that will use both prov1 and prov2 via a multi provider.
        let task_handle = tokio::spawn(async move {
            let request: SuggestionRequest = Faker.fake();
            multi.suggest(request).await.unwrap();
            // signal that the request has finished
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

    #[tokio::test]
    async fn test_reconfigure_optimisitc() {
        let prov_fixed = FixedProvider {
            value: "foo".to_owned(),
        };
        let prov_null = NullProvider;
        let mut provider = Multi::new(vec![Box::new(prov_fixed), Box::new(prov_null)]);

        // This won't be called as `DelayProvider::reconfigure()` will always succeed.
        let make_fresh: MakeFreshType = Box::new(move |fresh_config: SuggestionProviderConfig| {
            let provider: Box<dyn SuggestionProvider> = match fresh_config {
                SuggestionProviderConfig::Fixed(config) => Box::new(FixedProvider {
                    value: config.value,
                }),
                SuggestionProviderConfig::Null => Box::new(NullProvider),
                _ => unreachable!(),
            };
            ready(Ok(provider)).boxed()
        });

        let value = serde_json::to_value(MultiplexerConfig {
            providers: vec![
                SuggestionProviderConfig::Fixed(FixedConfig {
                    value: "bar".to_owned(),
                }),
                SuggestionProviderConfig::Null,
            ],
        })
        .expect("failed to serialize");
        provider
            .reconfigure(value, &make_fresh)
            .await
            .expect("failed to reconfigure");
        assert_eq!(provider.providers.len(), 2);

        let response = provider
            .suggest(Faker.fake())
            .await
            .expect("failed to suggest");
        assert_eq!(response.suggestions.len(), 1);
        assert_eq!(response.suggestions[0].title, "bar");
    }

    #[tokio::test]
    async fn test_reconfigure_pessimisitc() {
        let prov_fixed = FixedProvider {
            value: "foo".to_owned(),
        };
        let prov_null = NullProvider;
        let mut provider = Multi::new(vec![Box::new(prov_fixed), Box::new(prov_null)]);

        // This won't be called as `DelayProvider::reconfigure()` will always succeed.
        let make_fresh: MakeFreshType = Box::new(move |fresh_config: SuggestionProviderConfig| {
            let provider: Box<dyn SuggestionProvider> = match fresh_config {
                SuggestionProviderConfig::Fixed(config) => Box::new(FixedProvider {
                    value: config.value,
                }),
                SuggestionProviderConfig::Null => Box::new(NullProvider),
                _ => unreachable!(),
            };
            ready(Ok(provider)).boxed()
        });

        let value = serde_json::to_value(MultiplexerConfig {
            providers: vec![
                SuggestionProviderConfig::Fixed(FixedConfig {
                    value: "bar".to_owned(),
                }),
                SuggestionProviderConfig::Fixed(FixedConfig {
                    value: "baz".to_owned(),
                }),
                SuggestionProviderConfig::Null,
            ],
        })
        .expect("failed to serialize");
        provider
            .reconfigure(value, &make_fresh)
            .await
            .expect("failed to reconfigure");
        assert_eq!(provider.providers.len(), 3);

        let response = provider
            .suggest(Faker.fake())
            .await
            .expect("failed to suggest");
        assert_eq!(response.suggestions.len(), 2);
        assert_eq!(response.suggestions[0].title, "bar");
        assert_eq!(response.suggestions[1].title, "baz");
    }
}
