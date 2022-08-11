#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! Suggestion backends for [Merino](../merino/index.html).
//!
pub mod device_info;
mod domain;
pub mod metrics;

use std::hash::Hash;
use std::ops::Range;
use std::pin::Pin;
use std::time::Duration;
use std::{fmt::Debug, future::Future};

use crate::device_info::DeviceInfo;
pub use crate::domain::{CacheInputs, Proportion};
use actix_web::http::header::{AcceptLanguage, LanguageTag, Preference, QualityItem};
use anyhow::Context;
use async_trait::async_trait;
use fake::{
    faker::{
        address::en::{CityName, CountryCode, StateAbbr},
        lorem::en::{Word, Words},
    },
    Fake, Faker,
};
use http::Uri;
use merino_settings::SuggestionProviderConfig;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use thiserror::Error;

/// A provider that never provides any suggestions.

/// The range of major Firefox version numbers to use for testing.
pub const FIREFOX_TEST_VERSIONS: Range<u32> = 70..95;

/// A request for suggestions.
#[derive(Debug, Clone, Hash, Serialize)]
pub struct SuggestionRequest {
    /// The text typed by the user.
    pub query: String,

    /// Whether or not the request indicated support for English.
    pub accepts_english: bool,

    /// Country in ISO 3166-1 alpha-2 format, such as "MX" for Mexico or "IT" for Italy.
    pub country: Option<String>,

    /// Region/region (e.g. a US state) in ISO 3166-2 format, such as "QC"
    /// for Quebec (with country = "CA") or "TX" for Texas (with country = "US").
    pub region: Option<String>,

    /// The Designated Market Area code, as defined by [Nielsen]. Only defined in the US.
    ///
    /// [Nielsen]: https://www.nielsen.com/us/en/contact-us/intl-campaigns/dma-maps/
    pub dma: Option<u16>,

    /// City, listed by name such as "Portland" or "Berlin".
    pub city: Option<String>,

    /// The user agent of the request, including OS family, device form factor, and major Firefox
    /// version number.
    pub device_info: DeviceInfo,

    /// Client Variant strings typed by the user.
    pub client_variants: Option<Vec<String>>,
}

impl<F> fake::Dummy<F> for SuggestionRequest {
    fn dummy_with_rng<R: rand::Rng + ?Sized>(_config: &F, rng: &mut R) -> Self {
        Self {
            query: Words(1..10).fake_with_rng::<Vec<String>, R>(rng).join(" "),
            accepts_english: Faker.fake(),
            country: Some(CountryCode().fake::<String>()),
            region: Some(StateAbbr().fake::<String>()),
            dma: Some(rng.gen_range(100_u16..1000)),
            city: Some(CityName().fake::<String>()),
            device_info: Faker.fake(),
            client_variants: Some(Words(1..10).fake_with_rng::<Vec<String>, R>(rng)),
        }
    }
}

/// A response of suggestions, along with related metadata.
#[derive(Clone, Debug)]
pub struct SuggestionResponse {
    /// The relation of this response to the cache it came from, if any.
    pub cache_status: CacheStatus,

    /// The remaining time the response is valid, if applicable. If `None`, their
    /// is no recommended TTL value. Caching layers may provide one if
    /// appropriate. No value should be cached forever.
    pub cache_ttl: Option<Duration>,

    /// The suggestions to provide to the user.
    pub suggestions: Vec<Suggestion>,
}

impl SuggestionResponse {
    /// Create a new suggestion response containing the given suggestions and cache status.
    ///
    /// The `json` field will be `None`.
    pub fn new(suggestions: Vec<Suggestion>) -> Self {
        Self {
            suggestions,
            cache_status: CacheStatus::NoCache,
            cache_ttl: None,
        }
    }

    /// Change the cache status of this response.
    pub fn with_cache_status(mut self, cache_status: CacheStatus) -> Self {
        self.cache_status = cache_status;
        self
    }

    /// Change the cache TTL of this response.
    pub fn with_cache_ttl(mut self, cache_ttl: Duration) -> Self {
        self.cache_ttl = Some(cache_ttl);
        self
    }
}

impl<F> fake::Dummy<F> for SuggestionResponse {
    fn dummy_with_rng<R: rand::Rng + ?Sized>(_config: &F, rng: &mut R) -> Self {
        SuggestionResponse {
            cache_status: CacheStatus::NoCache,
            cache_ttl: None,
            suggestions: std::iter::repeat_with(|| Faker.fake())
                .take(rng.gen_range(0..=5))
                .collect(),
        }
    }
}

/// The relation between an object and a cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStatus {
    /// The object was pulled fresh from the cache.
    Hit,
    /// The object was not available from the cache, and was regenerated.
    Miss,
    /// No cache was consulted for this response.
    NoCache,
    /// The response is made of suggestions from multiple sources that have varying cache status.
    Mixed,
    /// There was an error while retrieving data from the cache.
    Error,
}

impl ToString for CacheStatus {
    fn to_string(&self) -> String {
        match self {
            CacheStatus::Hit => "hit",
            CacheStatus::Miss => "miss",
            CacheStatus::NoCache => "no-cache",
            CacheStatus::Mixed => "mixed",
            CacheStatus::Error => "error",
        }
        .to_string()
    }
}

/// A suggestion to provide to a user.
#[serde_as]
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Suggestion {
    /// The content provider ID of the suggestion.
    pub id: u32,

    /// If this suggestion can be matched with partial keywords this is the full
    /// keyword of the suggestion.
    pub full_keyword: String,

    /// The title to display to the user.
    pub title: String,

    /// The URL to send the user to if they select this suggestion.
    #[serde_as(as = "DisplayFromStr")]
    pub url: Uri,

    /// The URL to notify when this keyword is presented to a user.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub impression_url: Option<Uri>,

    /// The URL to notify when this keyword is clicked on by a user.
    #[serde_as(as = "Option<DisplayFromStr>")]
    #[serde(default)]
    pub click_url: Option<Uri>,

    /// The name of the provider associated with this suggestion.
    pub provider: String,

    /// The name of the advertiser associated with this suggestion.
    pub advertiser: String,

    /// Whether this suggestion is sponsored.
    pub is_sponsored: bool,

    /// The URL of the icon to show along side this suggestion.
    #[serde_as(as = "DisplayFromStr")]
    pub icon: Uri,

    /// A value used to compare suggestions. When choosing a suggestion to show
    /// the user, higher scored suggestions are preferred. Should range from 0.0
    /// to 1.0.
    ///
    /// Note that Firefox uses a static value of 0.2 for Remote Settings
    /// provided suggestions.
    pub score: Proportion,
}

impl<F> fake::Dummy<F> for Suggestion {
    fn dummy_with_rng<R: rand::Rng + ?Sized>(_config: &F, rng: &mut R) -> Self {
        Self {
            id: Faker.fake(),
            full_keyword: Word().fake_with_rng(rng),
            title: Words(3..5).fake_with_rng::<Vec<String>, R>(rng).join(" "),
            url: fake_example_url(rng),
            impression_url: Some(fake_example_url(rng)),
            click_url: Some(fake_example_url(rng)),
            provider: Words(2..4).fake_with_rng::<Vec<String>, R>(rng).join(" "),
            advertiser: Words(2..4).fake_with_rng::<Vec<String>, R>(rng).join(" "),
            is_sponsored: rng.gen(),
            icon: fake_example_url(rng),
            score: rng.gen(),
        }
    }
}

/// Helper to generate a URL to use for testing, of the form
/// "https://example.com/fake#some-random-words".
fn fake_example_url<R: rand::Rng + ?Sized>(rng: &mut R) -> Uri {
    Uri::builder()
        .scheme("https")
        .authority("example.com")
        .path_and_query(format!(
            "/fake#{}",
            Words(2..5).fake_with_rng::<Vec<String>, R>(rng).join("-")
        ))
        .build()
        .unwrap()
}

/// A backend that can provide suggestions for queries.
#[async_trait]
pub trait SuggestionProvider: Send + Sync {
    /// An operator-visible name for this suggestion provider.
    fn name(&self) -> String;

    /// Provide suggested results for `query`.
    async fn suggest(&self, query: SuggestionRequest) -> Result<SuggestionResponse, SuggestError>;

    /// Return if this provider is null and can be ignored. Providers that set
    /// this to true should be ignored in any place where suggestions are
    /// needed. Providers with this set to true likely only serve as a blank
    /// space where we may need a provider but can't otherwise supply one.
    fn is_null(&self) -> bool {
        false
    }

    /// Generate a set of cache inputs for a given query specific to this
    /// provider. Any property of the query that affects how suggestions are
    /// generated should be included.
    ///
    /// By default, all properties of the query are used, but providers should
    /// narrow this to a smaller scope.
    fn cache_inputs(&self, req: &SuggestionRequest, cache_inputs: &mut dyn CacheInputs) {
        cache_inputs.add(req.query.as_bytes());
        cache_inputs.add(&[req.accepts_english as u8]);
        cache_inputs.add(req.country.as_deref().unwrap_or("<none>").as_bytes());
        cache_inputs.add(req.region.as_deref().unwrap_or("<none>").as_bytes());
        cache_inputs.add(&req.dma.map_or([0xFF, 0xFF], u16::to_be_bytes));
        cache_inputs.add(req.city.as_deref().unwrap_or("<none>").as_bytes());
        cache_inputs.add(req.device_info.to_string().as_bytes());
    }

    /// Use `Self::cache_inputs` to generate a single cache key. This function
    /// should not normally be overridden by provider implementations.
    fn cache_key(&self, req: &SuggestionRequest) -> String {
        let mut cache_inputs = blake3::Hasher::new();
        cache_inputs.add(self.name().as_bytes());
        self.cache_inputs(req, &mut cache_inputs);
        format!("provider:v1:{}", cache_inputs.hash())
    }

    /// Reconfigure the provider, using a new configuration object. State should
    /// be preserved if possible.
    ///
    /// The parameter `make_fresh` can be used to make a new provider from a
    /// configuration, such as if a inner provider must be thrown away and
    /// recreated.
    async fn reconfigure(
        &mut self,
        new_config: serde_json::Value,
        make_fresh: &MakeFreshType,
    ) -> Result<(), SetupError>;
}

/// A type that represents an object-safe function that can be passed to
/// suggestion reconfigure methods to make new inner providers.
pub type MakeFreshType = Box<
    dyn Send
        + Sync
        + Fn(
            SuggestionProviderConfig,
        ) -> Pin<
            Box<
                (dyn Send
                     + Future<Output = Result<Box<dyn SuggestionProvider>, SetupError>>
                     + 'static),
            >,
        >,
>;

/// Errors that may occur while setting up the provider.
#[derive(Debug, Error)]
#[allow(missing_docs, clippy::missing_docs_in_private_items)]
pub enum SetupError {
    #[error("This suggestions provider cannot be used with the current Merino configuration")]
    InvalidConfiguration(#[source] anyhow::Error),

    #[error("There was a network error while setting up this suggestions provider")]
    Network(#[source] anyhow::Error),

    #[error("There was a local I/O error while setting up this suggestion provider")]
    Io(#[source] anyhow::Error),

    #[error("Required data was not in the expected format")]
    Format(#[source] anyhow::Error),

    #[error("An unexpected state was encountered")]
    Internal(#[source] anyhow::Error),
}

/// Errors that may occur while querying for suggestions.
#[derive(Debug, Error)]
#[allow(missing_docs, clippy::missing_docs_in_private_items)]
pub enum SuggestError {
    #[error("There was a network error while providing suggestions: {0}")]
    Network(#[source] anyhow::Error),

    #[error("There was an error serializing the suggestions: {0}")]
    Serialization(#[source] serde_json::Error),

    #[error("There was an internal error in the suggestion provider: {0}")]
    Internal(#[source] anyhow::Error),
}

/// Languages supported by the client.
#[derive(Debug, PartialEq, Eq)]
pub struct SupportedLanguages(pub AcceptLanguage);

impl SupportedLanguages {
    /// Returns an instance of [`SupportedLanguages`] that includes every language.
    pub fn wildcard() -> Self {
        Self(AcceptLanguage(vec![QualityItem::max(Preference::Any)]))
    }

    /// Specify whether `self` includes the language specified by the given language and region.
    pub fn includes(&self, language_tag: LanguageTag) -> bool {
        self.0.iter().any(|quality_item| {
            language_tag.matches(&match &quality_item.item {
                Preference::Any => return true,
                Preference::Specific(item) => item.to_owned(),
            })
        })
    }
}

/// A basic provider that never returns any suggestions.
pub struct NullProvider;

#[async_trait]
impl SuggestionProvider for NullProvider {
    fn name(&self) -> String {
        "NullProvider".into()
    }

    fn cache_inputs(&self, _req: &SuggestionRequest, _hasher: &mut dyn CacheInputs) {
        // No property of req will change the response
    }

    fn is_null(&self) -> bool {
        true
    }

    async fn suggest(&self, _query: SuggestionRequest) -> Result<SuggestionResponse, SuggestError> {
        Ok(SuggestionResponse::new(vec![]))
    }

    async fn reconfigure(
        &mut self,
        new_config: serde_json::Value,
        _make_fresh: &MakeFreshType,
    ) -> Result<(), SetupError> {
        // make sure this is the right kind of config
        convert_config::<SuggestionProviderConfig>(new_config).map(|_| ())
    }
}

/// Convert a [`serde_json::Value`] into an appropriate provider configuration object.
/// # Errors
/// If the `Value` passed does not conform to the expected schema of the config,
/// a `SetupError::InvalidConfiguration` error will be returned.
pub fn convert_config<T: DeserializeOwned>(config: serde_json::Value) -> Result<T, SetupError> {
    serde_json::from_value::<T>(config)
        .context("loading provider config")
        .map_err(SetupError::InvalidConfiguration)
}

/// Reconfigure the passed provider, or if there is a problem reconfiguring it,
/// replace it with a newly created provider.
/// # Errors
/// If there is an error serializing the config, no changes will be made and
/// that error will be returned. If there is an error reconfiguring the original
/// provider, a new provider will be created and written to the reference for
/// the original provider, overwriting the original. The error that resulted
/// from reconfigure will be logged as a warning. If there is an error during
/// while creating the new provider, the original provider will not be
/// overwritten, and may be in an inconsistent state.
pub async fn reconfigure_or_remake(
    provider: &mut Box<dyn SuggestionProvider>,
    new_config: SuggestionProviderConfig,
    make_fresh: &MakeFreshType,
) -> Result<(), SetupError> {
    let serialized_config = serde_json::to_value(new_config.clone())
        .context("serializing provider config")
        .map_err(SetupError::InvalidConfiguration)?;

    if let Err(error) = provider.reconfigure(serialized_config, make_fresh).await {
        tracing::warn!(
            r#type = "suggest-traits.reconfigure-or-remake.reconfigure-error",
            ?error,
            "Could not reconfigure provider in place"
        );
        *provider = make_fresh(new_config).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::header::QualityItem;

    #[test]
    fn supported_languages_includes_example() {
        let supported_languages = SupportedLanguages(AcceptLanguage(vec![
            QualityItem::max("en-CA".parse().unwrap()),
            QualityItem::max("fr".parse().unwrap()),
        ]));

        // Includes en-CA
        assert!(supported_languages.includes(LanguageTag::parse("en-CA").unwrap()));

        // Includes en
        assert!(supported_languages.includes(LanguageTag::parse("en").unwrap()));

        // Does not include en-GB
        assert!(!supported_languages.includes(LanguageTag::parse("en-GB").unwrap()));

        // Includes fr
        assert!(supported_languages.includes(LanguageTag::parse("fr").unwrap()));

        // Does not include fr-CH
        assert!(!supported_languages.includes(LanguageTag::parse("fr-CH").unwrap()));

        let supported_languages =
            SupportedLanguages(AcceptLanguage(vec![QualityItem::max("*".parse().unwrap())]));

        // Includes en-CA
        assert!(supported_languages.includes(LanguageTag::parse("en-CA").unwrap()));

        // Includes en
        assert!(supported_languages.includes(LanguageTag::parse("en").unwrap()));

        // Includes fr-CH
        assert!(supported_languages.includes(LanguageTag::parse("fr-CH").unwrap()));
    }

    /// A test provider that only considers the request query for caching.
    struct TestProvider;

    #[async_trait]
    impl SuggestionProvider for TestProvider {
        fn name(&self) -> String {
            "test".to_string()
        }

        async fn suggest(
            &self,
            _query: SuggestionRequest,
        ) -> Result<SuggestionResponse, SuggestError> {
            unimplemented!()
        }

        fn cache_inputs(&self, req: &SuggestionRequest, cache_inputs: &mut dyn CacheInputs) {
            cache_inputs.add(req.query.as_bytes());
        }

        async fn reconfigure(
            &mut self,
            _new_config: serde_json::Value,
            _make_fresh: &MakeFreshType,
        ) -> Result<(), SetupError> {
            unimplemented!()
        }
    }

    #[test]
    fn cache_key_only_considers_included_inputs() {
        // 2x2 matrix: one axis is `query from {a, b}`, the other is `accepts_english from {false, true}`
        let request1 = SuggestionRequest {
            query: "a".to_string(),
            accepts_english: true,
            ..Faker.fake()
        };
        let request2 = SuggestionRequest {
            query: "a".to_string(),
            accepts_english: false,
            ..request1.clone()
        };
        let request3 = SuggestionRequest {
            query: "b".to_string(),
            accepts_english: true,
            ..request1.clone()
        };
        let request4 = SuggestionRequest {
            query: "b".to_string(),
            accepts_english: false,
            ..request1.clone()
        };

        let provider = TestProvider;
        // same `query` (a), different `accepts_english`.
        assert_eq!(provider.cache_key(&request1), provider.cache_key(&request2));
        // same `query` (b), different `accepts_english`.
        assert_eq!(provider.cache_key(&request3), provider.cache_key(&request4));
        // different query, same accepts_english
        assert_ne!(provider.cache_key(&request1), provider.cache_key(&request3));
    }
}
