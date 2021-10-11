//! A suggestion provider that filters suggestions from a subprovider.

use crate::{SetupError, SuggestionProvider, SuggestionResponse};
use anyhow::anyhow;
use async_trait::async_trait;
use regex::{Regex, RegexBuilder};
use std::collections::HashMap;

/// A combinator provider that filters the results from the wrapped provider
/// using a blocklist from the settings.
pub struct KeywordFilterProvider {
    /// A map linking the filter identifiers to their compiled regexes.
    blocklist: HashMap<String, regex::Regex>,

    /// The provider to pull suggestions from.
    inner: Box<dyn SuggestionProvider>,
}

impl KeywordFilterProvider {
    /// Construct a new, boxed filter provider.
    pub fn new_boxed(
        blocklist: HashMap<String, String>,
        inner: Box<dyn SuggestionProvider>,
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
        for filter in &self.blocklist {
            let initial_suggestions = results.suggestions.len();
            results.suggestions.retain(|r| !filter.1.is_match(&r.title));
            // TODO: increment metric
            println!(
                "**** DEBUG - Should record {:?} matches",
                (initial_suggestions - results.suggestions.len())
            );
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
        blocklist.insert("filter_1".to_string(), regex::Regex::new("test").unwrap());

        let filter_provider = KeywordFilterProvider {
            blocklist,
            inner: Box::new(TestSuggestionsProvider()),
        };
        let res = filter_provider
            .suggest(Faker.fake())
            .await
            .expect("failed to get suggestion");
        println!("**** DEBUG - Test Results {:?}", res);

        assert_eq!(res.suggestions.len(), 1);
        assert_eq!(res.suggestions[0].provider, "TestSuggestionsProvider()");
        assert_eq!(res.suggestions[0].title, "A suggestion that goes through");

        // TODO: test data collection
    }

    #[tokio::test]
    async fn test_provider_all_filtered() {
        let mut blocklist = HashMap::new();
        blocklist.insert("filter_1".to_string(), regex::Regex::new("test").unwrap());
        blocklist.insert(
            "filter_2".to_string(),
            regex::Regex::new("through").unwrap(),
        );

        let filter_provider = KeywordFilterProvider {
            blocklist,
            inner: Box::new(TestSuggestionsProvider()),
        };
        let res = filter_provider
            .suggest(Faker.fake())
            .await
            .expect("failed to get suggestion");
        println!("**** DEBUG - Test Results {:?}", res);

        assert_eq!(res.suggestions.len(), 0);

        // TODO: test data collection
    }

    #[tokio::test]
    async fn test_provider_nothing_filtered() {
        let mut blocklist = HashMap::new();
        blocklist.insert(
            "filter_1".to_string(),
            regex::Regex::new("no-match").unwrap(),
        );

        let filter_provider = KeywordFilterProvider {
            blocklist,
            inner: Box::new(TestSuggestionsProvider()),
        };
        let res = filter_provider
            .suggest(Faker.fake())
            .await
            .expect("failed to get suggestion");
        println!("**** DEBUG - Test Results {:?}", res);

        assert_eq!(res.suggestions.len(), 2);

        // TODO: test data collection
    }
}
