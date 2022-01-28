//! A suggestion provider switches between a matching and default provider based on the client variant string.
use crate::{CacheInputs, SuggestError, SuggestionProvider, SuggestionRequest, SuggestionResponse};
use async_trait::async_trait;

/// A provider that gives suggestions base
pub struct ClientVariantFilterProvider {
    /// Provider to use for suggestions if there is a client variant match
    matching_provider: Box<dyn SuggestionProvider>,
    /// Provider to use for suggestions if there isn't a client variant match
    default_provider: Box<dyn SuggestionProvider>,
    /// String use to match with client variants from suggest requests
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
            "ClientVariant(matching:{}, default:{}, client_variant match: {})",
            self.matching_provider.name(),
            self.default_provider.name(),
            self.client_variant,
        )
    }

    async fn suggest(
        &self,
        request: SuggestionRequest,
    ) -> Result<SuggestionResponse, SuggestError> {
        let req = request.clone();
        let provider = if req
            .client_variants
            .as_ref()
            .map_or(false, |cv| cv.contains(&self.client_variant))
        {
            &self.matching_provider
        } else {
            &self.default_provider
        };
        let results = provider.suggest(request).await?;
        Ok(results)
    }

    fn cache_inputs(&self, request: &SuggestionRequest, cache_inputs: &mut dyn CacheInputs) {
        self.matching_provider.cache_inputs(request, cache_inputs);
        self.default_provider.cache_inputs(request, cache_inputs);

        if request
            .client_variants
            .as_ref()
            .map_or(false, |cv| cv.contains(&self.client_variant))
        {
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
        ClientVariantFilterProvider, FixedProvider, SuggestionProvider, SuggestionRequest,
    };
    use fake::{Fake, Faker};

    #[tokio::test]
    async fn test_provider_uses_default_without_client_variants() {
        let client_variant_filter_provider = ClientVariantFilterProvider::new_boxed(
            Box::new(FixedProvider {
                value: "Matching Provider".to_string(),
            }),
            Box::new(FixedProvider {
                value: "Default Provider".to_string(),
            }),
            "test".to_string(),
        );

        let res = client_variant_filter_provider
            .suggest(Faker.fake())
            .await
            .expect("failed to get suggestion");

        assert_eq!(
            res.suggestions[0].provider,
            "FixedProvider(Default Provider)"
        );
        assert_eq!(res.suggestions[0].title, "Default Provider");
    }

    #[tokio::test]
    async fn test_provider_uses_matching_with_client_variants() {
        let client_variant_filter_provider = ClientVariantFilterProvider::new_boxed(
            Box::new(FixedProvider {
                value: "Matching Provider".to_string(),
            }),
            Box::new(FixedProvider {
                value: "Default Provider".to_string(),
            }),
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
            "FixedProvider(Matching Provider)"
        );
        assert_eq!(res.suggestions[0].title, "Matching Provider");
    }
}
