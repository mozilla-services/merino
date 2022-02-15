//! Tools to make sure providers don't  cache_status: todo!(), cache_ttl: todo!(), suggestions: todo!() take excessive amounts of time.

use async_trait::async_trait;
use merino_settings::providers::TimeoutConfig;
use merino_suggest_traits::{
    convert_config, reconfigure_or_remake, CacheInputs, CacheStatus, MakeFreshType, SetupError,
    SuggestError, SuggestionProvider, SuggestionRequest, SuggestionResponse,
};
use std::time::Duration;

/// A combinator provider that returns an empty set of suggestions if the wrapped provider takes too long.
pub struct TimeoutProvider {
    /// The time to wait before returning an empty set of suggestions.
    max_time: Duration,

    /// The provider to pull suggestions from.
    inner: Box<dyn SuggestionProvider>,
}

impl TimeoutProvider {
    /// Construct a new, boxed timeout provider.
    #[must_use]
    pub fn new_boxed(config: TimeoutConfig, inner: Box<dyn SuggestionProvider>) -> Box<Self> {
        Box::new(Self {
            max_time: config.max_time,
            inner,
        })
    }
}

#[async_trait]
impl SuggestionProvider for TimeoutProvider {
    fn name(&self) -> String {
        format!("timeout({})", self.inner.name())
    }

    fn cache_inputs(&self, req: &SuggestionRequest, cache_inputs: &mut dyn CacheInputs) {
        self.inner.cache_inputs(req, cache_inputs);
    }

    async fn suggest(&self, query: SuggestionRequest) -> Result<SuggestionResponse, SuggestError> {
        let inner_fut = self.inner.suggest(query);
        let timeout = tokio::time::timeout(self.max_time, inner_fut).await;
        timeout.unwrap_or_else(|_timeout_elapsed| {
            Ok(SuggestionResponse {
                cache_status: CacheStatus::Error,
                cache_ttl: None,
                suggestions: vec![],
            })
        })
    }

    async fn reconfigure(
        &mut self,
        new_config: serde_json::Value,
        make_fresh: &MakeFreshType,
    ) -> Result<(), SetupError> {
        let new_config: TimeoutConfig = convert_config(new_config)?;
        reconfigure_or_remake(&mut self.inner, *new_config.inner, make_fresh).await?;
        self.max_time = new_config.max_time;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::TimeoutProvider;
    use async_trait::async_trait;
    use fake::{Fake, Faker};
    use futures::{future::ready, FutureExt};
    use merino_settings::providers::{SuggestionProviderConfig, TimeoutConfig};
    use merino_suggest_traits::{
        CacheStatus, MakeFreshType, SetupError, SuggestError, Suggestion, SuggestionProvider,
        SuggestionRequest, SuggestionResponse,
    };
    use std::time::Duration;

    struct DelayProvider(Duration);

    #[async_trait]
    impl SuggestionProvider for DelayProvider {
        fn name(&self) -> String {
            format!("DelayProvider({}ms)", self.0.as_millis())
        }

        async fn suggest(
            &self,
            _query: SuggestionRequest,
        ) -> Result<SuggestionResponse, SuggestError> {
            tokio::time::sleep(self.0).await;
            Ok(SuggestionResponse {
                cache_status: CacheStatus::NoCache,
                cache_ttl: None,
                suggestions: vec![Suggestion {
                    provider: self.name(),
                    ..Faker.fake()
                }],
            })
        }

        async fn reconfigure(
            &mut self,
            _new_config: serde_json::Value,
            _make_fresh: &MakeFreshType,
        ) -> Result<(), SetupError> {
            // No-op
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_provider_too_slow() {
        let timeout_provider = TimeoutProvider {
            max_time: Duration::from_millis(10),
            inner: Box::new(DelayProvider(Duration::from_millis(1000))),
        };
        let res = timeout_provider
            .suggest(Faker.fake())
            .await
            .expect("failed to get suggestion");
        assert_eq!(res.suggestions, vec![]);
    }

    #[tokio::test]
    async fn test_provider_fast_enough() {
        let timeout_provider = TimeoutProvider {
            max_time: Duration::from_millis(1000),
            inner: Box::new(DelayProvider(Duration::from_millis(10))),
        };
        let res = timeout_provider
            .suggest(Faker.fake())
            .await
            .expect("failed to get suggestion");
        assert_eq!(res.suggestions.len(), 1);
        assert_eq!(res.suggestions[0].provider, "DelayProvider(10ms)");
    }

    #[tokio::test]
    async fn test_reconfigure() {
        let mut timeout_provider = TimeoutProvider {
            max_time: Duration::from_millis(1000),
            inner: Box::new(DelayProvider(Duration::from_millis(10))),
        };

        // This won't be called as `DelayProvider::reconfigure()` will always succeed.
        let make_fresh: MakeFreshType = Box::new(move |_fresh_config: SuggestionProviderConfig| {
            ready(Ok(
                Box::new(DelayProvider(Duration::from_millis(1000))) as Box<dyn SuggestionProvider>
            ))
            .boxed()
        });

        // Reconfigure the outer provider to be the default.
        let value = serde_json::to_value(TimeoutConfig::default()).expect("failed to serialize");
        timeout_provider
            .reconfigure(value, &make_fresh)
            .await
            .expect("failed to reconfigure");
        assert_eq!(timeout_provider.max_time, TimeoutConfig::default().max_time);
    }
}
