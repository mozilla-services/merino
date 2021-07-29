//! Web handlers for the suggestions API.

use std::borrow::Cow;
use std::str;

use crate::errors::HandlerError;
use actix_web::{
    dev::Payload,
    get,
    web::{Data, Query, ServiceConfig},
    Error, FromRequest, HttpRequest, HttpResponse,
};
use anyhow::Result;
use futures_util::future::{self, Ready};
use merino_adm::remote_settings::RemoteSettingsSuggester;
use merino_settings::Settings;
use merino_suggest::{
    Language, LanguageIdentifier, Suggestion, SuggestionProvider, SuggestionRequest,
    SupportedLanguages, WikiFruit,
};

use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;
use tracing_futures::Instrument;

/// Configure a route to use the Suggest service.
pub fn configure(config: &mut ServiceConfig) {
    config
        .app_data(Data::new(SuggestionProviderRef(OnceCell::new())))
        .service(suggest);
}

/// A query passed to the API.
#[derive(Debug, Deserialize)]
struct SuggestQuery {
    /// The query to generate suggestions for.
    q: String,
}

/// The response the API generates.
#[derive(Debug, Serialize)]
struct SuggestResponse<'a> {
    /// A list of suggestions from the service.
    suggestions: &'a [Suggestion],
}

/// Suggest content in response to the queried text.
#[get("")]
#[tracing::instrument(skip(query, provider, settings))]
async fn suggest<'a>(
    query: Query<SuggestQuery>,
    provider: Data<SuggestionProviderRef<'a>>,
    languages: SupportedLanguagesWrapper,
    settings: Data<Settings>,
) -> Result<HttpResponse, HandlerError> {
    let provider = provider
        .get_or_try_init(settings.as_ref())
        .await
        .map_err(|error| {
            tracing::error!(
                ?error,
                r#type = "web.suggest.setup-error",
                "suggester error"
            );
            HandlerError::Internal
        })?;

    let suggestion_request = SuggestionRequest {
        query: Cow::from(query.into_inner().q),
        accepts_english: languages.0.includes("en", None),
    };

    let response = provider
        .suggest(suggestion_request)
        .await
        .map_err(|error| {
            tracing::error!(%error, r#type="web.suggest.error", "Error providing suggestions");
            HandlerError::Internal
        })?;

    tracing::debug!(
        r#type = "web.suggest.provided-count",
        suggestion_count = response.suggestions.len(),
        "Providing suggestions"
    );

    let res = HttpResponse::Ok()
        .append_header(("X-Cache", response.cache_status.to_string()))
        .json(SuggestResponse {
            suggestions: &response.suggestions,
        });

    Ok(res)
}

/// The SuggestionProvider stored in Actix's app_data.
struct SuggestionProviderRef<'a>(OnceCell<merino_suggest::Multi<'a>>);

impl<'a> SuggestionProviderRef<'a> {
    /// Get the provider, or create a new one if it doesn't exist.
    async fn get_or_try_init(
        &self,
        settings: &Settings,
    ) -> anyhow::Result<&merino_suggest::Multi<'a>> {
        let setup_span = tracing::info_span!("suggestion_provider_setup");
        self.0
            .get_or_try_init(|| {
                async {
                    let settings = settings;
                    tracing::info!(
                        r#type = "web.configuring-suggesters",
                        "Setting up suggestion providers"
                    );

                    /// The number of providers we expect to have, so we usually
                    /// don't have to re-allocate the vec.
                    const NUM_PROVIDERS: usize = 2;
                    let mut providers: Vec<Box<dyn SuggestionProvider + Send + Sync>> =
                        Vec::with_capacity(NUM_PROVIDERS);

                    if settings.providers.wiki_fruit.enabled {
                        let wikifruit = WikiFruit::new_boxed(settings)?;
                        providers.push(match settings.providers.wiki_fruit.cache {
                            merino_settings::CacheType::None => wikifruit,
                            merino_settings::CacheType::Redis => {
                                merino_cache::RedisSuggester::new_boxed(settings, *wikifruit)
                                    .await?
                            }
                        });
                    }

                    if settings.providers.adm_rs.enabled {
                        let adm_rs = RemoteSettingsSuggester::new_boxed(settings).await?;
                        providers.push(match settings.providers.adm_rs.cache {
                            merino_settings::CacheType::None => adm_rs,
                            merino_settings::CacheType::Redis => {
                                merino_cache::RedisSuggester::new_boxed(settings, *adm_rs).await?
                            }
                        });
                    }

                    let multi = merino_suggest::Multi::new(providers);
                    Ok(multi)
                }
                .instrument(setup_span)
            })
            .await
    }
}

/// A wrapper around `SupportedLanguages`.
#[derive(Debug, PartialEq)]
struct SupportedLanguagesWrapper(SupportedLanguages);

impl FromRequest for SupportedLanguagesWrapper {
    type Config = ();
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        /// Parse the quality value from a string of the form q=`<quality value>`.
        fn parse_quality_value(quality_value: &str) -> Result<f64, HandlerError> {
            let (_, weight_as_string) = quality_value
                .split_once('=')
                .ok_or(HandlerError::MalformedHeader("Accept-Language"))?;

            let weight = weight_as_string
                .parse::<f64>()
                .map_err(|_| HandlerError::MalformedHeader("Accept-Language"))?;

            if (0.0..=1.0).contains(&weight) {
                Ok(weight)
            } else {
                Err(HandlerError::MalformedHeader("Accept-Language"))
            }
        }

        /// Parse the Accept-Language HTTP header.
        fn parse_language(raw_language: &str) -> Result<Language, Error> {
            let (locale_or_wildcard, quality_value) =
                if let Some((language, quality_value)) = raw_language.split_once(';') {
                    let quality_value = Some(parse_quality_value(quality_value)?);

                    (language, quality_value)
                } else {
                    (raw_language, None)
                };

            let language = if locale_or_wildcard == "*" {
                Language {
                    language_identifier: LanguageIdentifier::Wildcard,
                    quality_value,
                }
            } else if let Some((language, region)) = locale_or_wildcard.split_once("-") {
                Language {
                    language_identifier: LanguageIdentifier::Locale {
                        language: language.to_lowercase(),
                        region: Some(region.to_lowercase()),
                    },
                    quality_value,
                }
            } else {
                Language {
                    language_identifier: LanguageIdentifier::Locale {
                        language: locale_or_wildcard.to_lowercase(),
                        region: None,
                    },
                    quality_value,
                }
            };

            Ok(language)
        }

        // A closure is used here to enable the usage of the `?` operator, making error handling
        // more ergonomic.
        let parse_header = || {
            let headers = req.headers();
            let header = match headers.get("Accept-Language") {
                Some(header) => header.to_str().map_err::<Self::Error, _>(|_| {
                    HandlerError::MalformedHeader("Accept-Language").into()
                }),
                None => return Ok(Self(SupportedLanguages::wildcard())),
            }?;

            let languages = header
                .split(", ")
                .map(parse_language)
                .collect::<Result<Vec<Language>, _>>()?;

            Ok(Self(SupportedLanguages(languages)))
        };

        future::ready(parse_header())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use actix_web::{dev::Payload, http::Method, test::TestRequest};

    const SUGGEST_URI: &str = "/api/v1/suggest";

    fn test_request_with_accept_language(accept_language: &str) -> HttpRequest {
        TestRequest::with_uri(SUGGEST_URI)
            .insert_header(("Accept-Language", accept_language))
            .method(Method::GET)
            .param("q", "asdf")
            .to_http_request()
    }

    #[actix_rt::test]
    async fn test_valid_accept_language_headers() {
        let mut payload = Payload::None;

        // Test single language without region
        let accept_language = "en";
        let req = test_request_with_accept_language(accept_language);
        let result = SupportedLanguagesWrapper::from_request(&req, &mut payload)
            .await
            .expect("Could not get result in test_valid_accept_language_headers");
        let expected_supported_languages_wrapper = {
            let language = Language {
                language_identifier: LanguageIdentifier::Locale {
                    language: "en".to_owned(),
                    region: None,
                },
                quality_value: None,
            };
            let supported_languages = SupportedLanguages(vec![language]);

            SupportedLanguagesWrapper(supported_languages)
        };

        assert_eq!(expected_supported_languages_wrapper, result);

        // Test single language with region
        let accept_language = "en-US";
        let req = test_request_with_accept_language(accept_language);
        let result = SupportedLanguagesWrapper::from_request(&req, &mut payload)
            .await
            .expect("Could not get result in test_valid_accept_language_headers");
        let expected_supported_languages_wrapper = {
            let language = Language {
                language_identifier: LanguageIdentifier::Locale {
                    language: "en".to_owned(),
                    region: Some("us".to_owned()),
                },
                quality_value: None,
            };
            let supported_languages = SupportedLanguages(vec![language]);

            SupportedLanguagesWrapper(supported_languages)
        };

        assert_eq!(expected_supported_languages_wrapper, result);

        // Test wildcard
        let accept_language = "*";
        let req = test_request_with_accept_language(accept_language);
        let result = SupportedLanguagesWrapper::from_request(&req, &mut payload)
            .await
            .expect("Could not get result in test_valid_accept_language_headers");
        let expected_supported_languages_wrapper =
            SupportedLanguagesWrapper(SupportedLanguages::wildcard());

        assert_eq!(expected_supported_languages_wrapper, result);

        // Test several languages with quality values
        let accept_language = "fr-CH, fr;q=0.9, en;q=0.8, de;q=0.7, *;q=0.5";
        let req = test_request_with_accept_language(accept_language);
        let result = SupportedLanguagesWrapper::from_request(&req, &mut payload)
            .await
            .expect("Could not get result in test_valid_accept_language_headers");
        let expected_supported_languages_wrapper = {
            let fr_ch = Language {
                language_identifier: LanguageIdentifier::Locale {
                    language: "fr".to_owned(),
                    region: Some("ch".to_owned()),
                },
                quality_value: None,
            };
            let fr = Language {
                language_identifier: LanguageIdentifier::Locale {
                    language: "fr".to_owned(),
                    region: None,
                },
                quality_value: Some(0.9),
            };
            let en = Language {
                language_identifier: LanguageIdentifier::Locale {
                    language: "en".to_owned(),
                    region: None,
                },
                quality_value: Some(0.8),
            };
            let de = Language {
                language_identifier: LanguageIdentifier::Locale {
                    language: "de".to_owned(),
                    region: None,
                },
                quality_value: Some(0.7),
            };
            let wildcard = Language {
                language_identifier: LanguageIdentifier::Wildcard,
                quality_value: Some(0.5),
            };

            SupportedLanguagesWrapper(SupportedLanguages(vec![fr_ch, fr, en, de, wildcard]))
        };

        assert_eq!(expected_supported_languages_wrapper, result);
    }

    #[actix_rt::test]
    async fn test_invalid_accept_language_headers() {
        let mut payload = Payload::None;

        // Malformed quality value
        let accept_language = "en-US;3";
        let req = test_request_with_accept_language(accept_language);
        let result = SupportedLanguagesWrapper::from_request(&req, &mut payload).await;

        assert_eq!(
            "Malformed header: Accept-Language",
            result.unwrap_err().to_string()
        );

        // Header with non-visible ASCII characters (\u{200B} is the zero-width space character)
        let accept_language = "\u{200B}";
        let req = test_request_with_accept_language(accept_language);
        let result = SupportedLanguagesWrapper::from_request(&req, &mut payload).await;

        assert_eq!(
            "Malformed header: Accept-Language",
            result.unwrap_err().to_string()
        );

        // Non-numeric quality value
        let accept_language = "en-US;q=one";
        let req = test_request_with_accept_language(accept_language);
        let result = SupportedLanguagesWrapper::from_request(&req, &mut payload).await;

        assert_eq!(
            "Malformed header: Accept-Language",
            result.unwrap_err().to_string()
        );
    }
}
