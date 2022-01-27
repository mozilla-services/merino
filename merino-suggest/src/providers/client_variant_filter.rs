use crate::{CacheInputs, SuggestError, SuggestionProvider, SuggestionRequest, SuggestionResponse};
use async_trait::async_trait;

/// A provider that gives suggestions base
pub struct ClientVariantFilterProvider {
    matching_provider: Box<dyn SuggestionProvider>,
    default_provider: Box<dyn SuggestionProvider>,
    client_variant: String,
}

impl ClientVariantFilterProvider {
    /// Create a boxed Client Variant Provider
    pub fn new_boxed(
        matching_provider: Box<dyn SuggestionProvider>,
        default_provider: Box<dyn SuggestionProvider>,
        client_variant: String,
    ) -> Box<Self> {
        Box::new(Self {
            matching_provider,
            default_provider,
            client_variant,
        })
    }
}

#[async_trait]
impl SuggestionProvider for ClientVariantFilterProvider {
    fn name(&self) -> String {
        format!(
            "ClientVariant(matching:{}, default:{})",
            self.matching_provider.name(),
            self.default_provider.name()
        )
    }

    async fn suggest(
        &self,
        request: SuggestionRequest,
    ) -> Result<SuggestionResponse, SuggestError> {
        let req = request.clone();
        let provider = if req.client_variants.unwrap_or(vec![]).contains(&self.client_variant) {
            &self.matching_provider
        } else {
            &self.default_provider
        };
        let results = provider
            .suggest(request)
            .await
            .unwrap_or_else(|_| SuggestionResponse::new(vec![]));
        Ok(results)
    }

    fn cache_inputs(&self, request: &SuggestionRequest, cache_inputs: &mut dyn CacheInputs) {
        self.matching_provider.cache_inputs(request, cache_inputs);
        self.default_provider.cache_inputs(request, cache_inputs);

        let req = request.clone();

        if req.client_variants.unwrap_or(vec![]).contains(&self.client_variant) {
            cache_inputs
                .add(format!("client_variant_match:{}=true", &self.client_variant).as_bytes());
        } else {
            cache_inputs
                .add(format!("client_variant_match:{}=false", &self.client_variant).as_bytes());
        };
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        CacheStatus, ClientVariantFilterProvider, SuggestError, Suggestion, SuggestionProvider,
        SuggestionRequest, SuggestionResponse,
    };
    use async_trait::async_trait;
    use fake::{Fake, Faker};

    struct TestMatchSuggestionsProvider();

    #[async_trait]
    impl SuggestionProvider for TestMatchSuggestionsProvider {
        fn name(&self) -> String {
            "TestMatchSuggestionsProvider()".to_string()
        }

        async fn suggest(
            &self,
            _query: SuggestionRequest,
        ) -> Result<SuggestionResponse, SuggestError> {
            Ok(SuggestionResponse {
                cache_status: CacheStatus::NoCache,
                cache_ttl: None,
                suggestions: vec![Suggestion {
                    provider: self.name(),
                    title: "matching test title".to_string(),
                    full_keyword: "matching".to_string(),
                    ..Faker.fake()
                }],
            })
        }
    }

    struct TestDefaultSuggestionsProvider();

    #[async_trait]
    impl SuggestionProvider for TestDefaultSuggestionsProvider {
        fn name(&self) -> String {
            "TestDefaultSuggestionsProvider()".to_string()
        }

        async fn suggest(
            &self,
            _query: SuggestionRequest,
        ) -> Result<SuggestionResponse, SuggestError> {
            Ok(SuggestionResponse {
                cache_status: CacheStatus::NoCache,
                cache_ttl: None,
                suggestions: vec![Suggestion {
                    provider: self.name(),
                    title: "A default test title".to_string(),
                    full_keyword: "default".to_string(),
                    ..Faker.fake()
                }],
            })
        }
    }

    #[tokio::test]
    async fn test_provider_uses_default_without_client_variants() {
        let client_variant_filter_provider = ClientVariantFilterProvider::new_boxed(
            Box::new(TestMatchSuggestionsProvider()),
            Box::new(TestDefaultSuggestionsProvider()),
            "test".to_string(),
        );

        let res = client_variant_filter_provider
            .suggest(Faker.fake())
            .await
            .expect("failed to get suggestion");

        assert_eq!(
            res.suggestions[0].provider,
            "TestDefaultSuggestionsProvider()"
        );
        assert_eq!(res.suggestions[0].title, "A default test title");
    }

    #[tokio::test]
    async fn test_provider_uses_matching_with_client_variants() {
        let client_variant_filter_provider = ClientVariantFilterProvider::new_boxed(
            Box::new(TestMatchSuggestionsProvider()),
            Box::new(TestDefaultSuggestionsProvider()),
            "test".to_string(),
        );

        let res = client_variant_filter_provider
            .suggest(SuggestionRequest {
                client_variants: Some(vec!["test".to_string()]),
                ..Faker.fake()
            })
            .await
            .expect("failed to get suggestion");

        assert_eq!(
            res.suggestions[0].provider,
            "TestMatchSuggestionsProvider()"
        );
        assert_eq!(res.suggestions[0].title, "matching test title");
    }
}
