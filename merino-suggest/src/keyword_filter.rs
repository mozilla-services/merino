//! A suggestion provider that filters suggestions from a subprovider.

use std::collections::HashMap;

use crate::{SetupError, SuggestionProvider, SuggestionResponse};
use anyhow::anyhow;
use async_trait::async_trait;
use cadence::{Counted, StatsdClient};
use regex::{Regex, RegexBuilder};

/// A combinator provider that filters the results from the wrapped provider
/// using a blocklist from the settings.
pub struct KeywordFilterProvider {
    /// A map linking the filter identifiers to their compiled regexes.
    blocklist: HashMap<String, regex::Regex>,

    /// The provider to pull suggestions from.
    inner: Box<dyn SuggestionProvider>,

    /// The Statsd client used to record statistics.
    metrics_client: StatsdClient,
}

impl KeywordFilterProvider {
    /// Construct a new, boxed filter provider.
    pub fn new_boxed(
        blocklist: HashMap<String, String>,
        inner: Box<dyn SuggestionProvider>,
        metrics_client: &StatsdClient,
    ) -> Result<Box<Self>, SetupError> {
        // Compile the provided blocklist regexes just once.
        let mut compiled_blocklist: HashMap<String, Regex> = HashMap::new();

        for (filter_id, filter_regex) in &blocklist {
            if let Ok(r) = RegexBuilder::new(filter_regex)
                .case_insensitive(true)
                .build()
            {
                compiled_blocklist.insert(filter_id.to_string(), r);
            } else {
                return Err(SetupError::InvalidConfiguration(anyhow!(
                    "KeywordFilterProvider failed to compile regex {} ({})",
                    filter_id,
                    filter_regex,
                )));
            }
        }

        Ok(Box::new(Self {
            blocklist: compiled_blocklist,
            inner,
            metrics_client: metrics_client.clone(),
        }))
    }
}

#[async_trait]
impl SuggestionProvider for KeywordFilterProvider {
    fn name(&self) -> String {
        format!("KeywordFilterProvider({})", self.inner.name())
    }

    async fn suggest(
        &self,
        query: crate::SuggestionRequest,
    ) -> Result<crate::SuggestionResponse, crate::SuggestError> {
        let mut results = self
            .inner
            .suggest(query)
            .await
            .unwrap_or_else(|_| SuggestionResponse::new(vec![]));

        // Some very naive filtering.
        for (filter_id, filter_regex) in &self.blocklist {
            let initial_suggestions = results.suggestions.len();
            results
                .suggestions
                .retain(|r| !filter_regex.is_match(&r.title));

            let matched_suggestions = initial_suggestions - results.suggestions.len();
            if matched_suggestions > 0 {
                self.metrics_client
                    // Note: the i64 conversion is required because `ToCounterValue` is
                    // not implemented for `usize`.
                    .count_with_tags("keywordfilter.match", matched_suggestions as i64)
                    .with_tag("id", filter_id)
                    .try_send()
                    .ok();
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        CacheStatus, KeywordFilterProvider, Suggestion, SuggestionProvider, SuggestionResponse,
    };
    use async_trait::async_trait;
    use cadence::{SpyMetricSink, StatsdClient};
    use fake::{Fake, Faker};
    use std::collections::HashMap;

    struct TestSuggestionsProvider();

    #[async_trait]
    impl SuggestionProvider for TestSuggestionsProvider {
        fn name(&self) -> String {
            "TestSuggestionsProvider()".to_string()
        }

        async fn suggest(
            &self,
            _query: crate::SuggestionRequest,
        ) -> Result<crate::SuggestionResponse, crate::SuggestError> {
            Ok(SuggestionResponse {
                cache_status: CacheStatus::NoCache,
                cache_ttl: None,
                suggestions: vec![
                    Suggestion {
                        provider: self.name(),
                        title: "A test title".to_string(),
                        full_keyword: "test".to_string(),
                        ..Faker.fake()
                    },
                    Suggestion {
                        provider: self.name(),
                        title: "A suggestion that goes through".to_string(),
                        full_keyword: "not matched".to_string(),
                        ..Faker.fake()
                    },
                ],
            })
        }
    }

    #[tokio::test]
    async fn test_provider_filters() {
        let mut blocklist = HashMap::new();
        blocklist.insert("filter_1".to_string(), "test".to_string());

        let (rx, sink) = SpyMetricSink::new();
        let metrics_client = StatsdClient::from_sink("merino-test", sink);

        let filter_provider = KeywordFilterProvider::new_boxed(
            blocklist,
            Box::new(TestSuggestionsProvider()),
            &metrics_client,
        )
        .expect("failed to create the keyword filter provider");

        let res = filter_provider
            .suggest(Faker.fake())
            .await
            .expect("failed to get suggestion");

        assert_eq!(res.suggestions.len(), 1);
        assert_eq!(res.suggestions[0].provider, "TestSuggestionsProvider()");
        assert_eq!(res.suggestions[0].title, "A suggestion that goes through");

        // Verify that the filtering was properly recorded.
        assert_eq!(rx.len(), 1);
        let sent = rx.recv().unwrap();
        assert_eq!(
            "merino-test.keywordfilter.match:1|c|#id:filter_1",
            String::from_utf8(sent).unwrap()
        );
    }

    #[tokio::test]
    async fn test_provider_all_filtered() {
        let mut blocklist = HashMap::new();
        blocklist.insert("filter_1".to_string(), "test".to_string());
        blocklist.insert("filter_2".to_string(), "through".to_string());

        let (rx, sink) = SpyMetricSink::new();
        let metrics_client = StatsdClient::from_sink("merino-test", sink);

        let filter_provider = KeywordFilterProvider::new_boxed(
            blocklist,
            Box::new(TestSuggestionsProvider()),
            &metrics_client,
        )
        .expect("failed to create the keyword filter provider");

        let res = filter_provider
            .suggest(Faker.fake())
            .await
            .expect("failed to get suggestion");

        assert_eq!(res.suggestions.len(), 0);

        // Verify that the filtering was properly recorded.
        assert_eq!(rx.len(), 2);
        let collected_data: Vec<String> = rx
            .iter()
            .take(2)
            .map(|x| String::from_utf8(x).unwrap())
            .collect();
        assert!(collected_data
            .contains(&"merino-test.keywordfilter.match:1|c|#id:filter_1".to_string()));
        assert!(collected_data
            .contains(&"merino-test.keywordfilter.match:1|c|#id:filter_2".to_string()));
    }

    #[tokio::test]
    async fn test_provider_nothing_filtered() {
        let mut blocklist = HashMap::new();
        blocklist.insert("filter_1".to_string(), "no-match".to_string());

        let (rx, sink) = SpyMetricSink::new();
        let metrics_client = StatsdClient::from_sink("merino-test", sink);

        let filter_provider = KeywordFilterProvider::new_boxed(
            blocklist,
            Box::new(TestSuggestionsProvider()),
            &metrics_client,
        )
        .expect("failed to create the keyword filter provider");

        let res = filter_provider
            .suggest(Faker.fake())
            .await
            .expect("failed to get suggestion");

        assert_eq!(res.suggestions.len(), 2);

        // Verify that nothing was recorded.
        assert!(rx.is_empty());
    }
}
