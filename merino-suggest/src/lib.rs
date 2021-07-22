#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! Suggestion backends for [Merino](../merino/index.html).

mod multi;
mod wikifruit;

use std::borrow::Cow;
use std::hash::Hash;
use std::time::Duration;

use async_trait::async_trait;
use http::Uri;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use thiserror::Error;

pub use crate::multi::Multi;
pub use crate::wikifruit::WikiFruit;

/// A request for suggestions.
#[derive(Debug, Clone, Hash)]
pub struct SuggestionRequest<'a> {
    /// The text typed by the user.
    pub query: Cow<'a, str>,
    /// Whether or not the request indicated support for English.
    pub supports_english: bool,
}

/// A response of suggestions, along with related metadata.
#[derive(Debug)]
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

/// The relation between an object and a cache.
#[derive(Debug, Clone, Copy, PartialEq)]
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
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Suggestion {
    /// The content provider ID of the suggestion.
    #[serde(rename = "block_id")]
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
    #[serde_as(as = "DisplayFromStr")]
    pub impression_url: Uri,

    /// The URL to notify when this keyword is clicked on by a user.
    #[serde_as(as = "DisplayFromStr")]
    pub click_url: Uri,

    /// The name of the advertiser associated with this suggestion.
    pub advertiser: String,

    /// Whether this suggestion is sponsored.
    pub is_sponsored: bool,

    /// The URL of the icon to show along side this suggestion.
    #[serde_as(as = "DisplayFromStr")]
    pub icon: Uri,
}

/// A backend that can provide suggestions for queries.
#[async_trait]
pub trait SuggestionProvider<'a> {
    /// An operator-visible name for this suggestion provider.
    fn name(&self) -> Cow<'a, str>;

    /// Provide suggested results for `query`.
    async fn suggest(
        &self,
        query: SuggestionRequest<'a>,
    ) -> Result<SuggestionResponse, SuggestError>;
}

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
}

/// Errors that may occur while querying for suggestions.
#[derive(Debug, Error)]
#[allow(missing_docs, clippy::missing_docs_in_private_items)]
pub enum SuggestError {
    #[error("There was a network error while providing suggestions")]
    Network(#[source] anyhow::Error),

    #[error("There was an error serializing the suggestions")]
    Serialization(#[source] serde_json::Error),
}

/// Languages supported by the client.
#[derive(Debug, PartialEq)]
pub struct SupportedLanguages(pub Vec<Language>);

impl SupportedLanguages {
    /// Create a new SupportedLanguages instance with a wildcard that has no quality value.
    pub fn wildcard() -> Self {
        let language = Language {
            language_identifier: LanguageIdentifier::Wildcard,
            quality_value: None,
        };

        Self(vec![language])
    }

    /// Specify whether `self` includes the language specified by the given language and region.
    pub fn includes(&self, language_query: &str, region_query: Option<&str>) -> bool {
        let region_matches = |supported_region| {
            match (supported_region, region_query) {
                // If the region query is None, the caller intends to match every region
                (_, None) => true,
                // If the region query is Some(_) but the supported region is None, the regions
                // don't match
                (None, Some(_)) => false,
                (Some(supported_region), Some(region_query)) => supported_region == region_query,
            }
        };

        self.0
            .iter()
            .any(|language| match &language.language_identifier {
                LanguageIdentifier::Locale { language, region } => {
                    language == language_query && region_matches(region.as_ref())
                }
                LanguageIdentifier::Wildcard => true,
            })
    }
}

/// A representation of a language, as given in the Accept-Language HTTP header.
#[derive(Debug, PartialEq)]
pub struct Language {
    /// Identifies a language (either a specific language or a wildcard).
    pub language_identifier: LanguageIdentifier,

    /// The quality value of the language.
    pub quality_value: Option<f64>,
}

/// An enum used to signify whether a `Language` refers to a specific language or a wildcard.
#[derive(Debug, PartialEq)]
pub enum LanguageIdentifier {
    /// A specific locale, consisting of a language code and optional country code.
    Locale {
        /// An ISO-639 language code.
        language: String,
        /// An ISO 3166-1 alpha-2 country code.
        region: Option<String>,
    },

    /// A wildcard, matching any language.
    Wildcard,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_languages_includes_example() {
        let supported_languages = {
            let en_ca = Language {
                language_identifier: LanguageIdentifier::Locale {
                    language: "en".to_owned(),
                    region: Some("ca".to_owned()),
                },
                quality_value: None,
            };

            let fr = Language {
                language_identifier: LanguageIdentifier::Locale {
                    language: "fr".to_owned(),
                    region: None,
                },
                quality_value: None,
            };

            SupportedLanguages(vec![en_ca, fr])
        };

        // Includes en-CA
        assert!(supported_languages.includes("en", Some("ca")));

        // Includes en
        assert!(supported_languages.includes("en", None));

        // Does not include en-GB
        assert!(!supported_languages.includes("en", Some("gb")));

        // Includes fr
        assert!(supported_languages.includes("fr", None));

        // Does not include fr-CH
        assert!(!supported_languages.includes("fr", Some("ch")));

        let supported_languages = {
            let wildcard = Language {
                language_identifier: LanguageIdentifier::Wildcard,
                quality_value: None,
            };

            SupportedLanguages(vec![wildcard])
        };

        // Includes en-CA
        assert!(supported_languages.includes("en", Some("ca")));

        // Includes en
        assert!(supported_languages.includes("en", None));

        // Includes fr-CH
        assert!(supported_languages.includes("fr", Some("ch")));
    }
}
