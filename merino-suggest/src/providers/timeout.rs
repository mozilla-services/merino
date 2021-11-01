//! Tools to make sure providers don't  cache_status: todo!(), cache_ttl: todo!(), suggestions: todo!() take excessive amounts of time.

use crate::{CacheStatus, SuggestionProvider, SuggestionResponse};
use async_trait::async_trait;
use merino_settings::providers::TimeoutConfig;
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
    pub fn new_boxed(config: &TimeoutConfig, inner: Box<dyn SuggestionProvider>) -> Box<Self> {
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

    async fn suggest(
        &self,
        query: crate::SuggestionRequest,
    ) -> Result<crate::SuggestionResponse, crate::SuggestError> {
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
}

#[cfg(test)]
mod tests {
    use crate::{CacheStatus, Suggestion, SuggestionProvider, SuggestionResponse, TimeoutProvider};
    use async_trait::async_trait;
    use fake::{Fake, Faker};
    use std::time::Duration;

    struct DelayProvider(Duration);

    #[async_trait]
    impl SuggestionProvider for DelayProvider {
        fn name(&self) -> String {
            format!("DelayProvider({}ms)", self.0.as_millis())
        }

        async fn suggest(
            &self,
            _query: crate::SuggestionRequest,
        ) -> Result<crate::SuggestionResponse, crate::SuggestError> {
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
}
