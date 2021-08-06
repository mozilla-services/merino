//! Types to extract merino data from requests.

use std::borrow::Cow;

use crate::errors::HandlerError;
use actix_web::{dev::Payload, web::Query, Error as ActixError, FromRequest, HttpRequest};
use actix_web_location::Location;
use futures_util::{
    future::{self, LocalBoxFuture, Ready},
    FutureExt,
};
use merino_suggest::{Language, LanguageIdentifier, SuggestionRequest, SupportedLanguages};
use serde::Deserialize;
use tokio::try_join;

/// An extractor for a [`merino_suggest::SuggestionRequest`].
pub struct SuggestionRequestWrapper<'a>(pub SuggestionRequest<'a>);

impl<'a> FromRequest for SuggestionRequestWrapper<'a> {
    type Config = ();

    type Error = ActixError;

    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        async move {
            // None of the requesters used below use payload, and getting `_payload` above into the closure is awkward.
            let mut fake_payload = Payload::None;

            /// try_join wants everything to have the same error type, and
            /// doesn't give a chance to map the error. This wrapper helps that.
            async fn loc_mapped_error(
                request: &HttpRequest,
                payload: &mut Payload,
            ) -> Result<Location, ActixError> {
                Location::from_request(request, payload)
                    .await
                    .map_err(ActixError::from)
            }

            // Retrieve all parts needed to make a SuggestionRequest concurrently.
            // `try_join` implicitly `.await`s.
            let (
                Query(SuggestQuery { q: query }),
                SupportedLanguagesWrapper(supported_languages),
                location,
            ) = try_join!(
                Query::from_request(&req, &mut fake_payload),
                SupportedLanguagesWrapper::from_request(&req, &mut fake_payload),
                loc_mapped_error(&req, &mut fake_payload),
            )?;

            Ok(Self(SuggestionRequest {
                query: query.into(),
                accepts_english: supported_languages.includes("en", None),
                country: location.country.map(Cow::from),
                region: location.region.map(Cow::from),
                dma: location.dma,
                city: location.city.map(Cow::from),
            }))
        }
        .boxed_local()
    }
}

/// A query passed to the API.
#[derive(Debug, Deserialize)]
struct SuggestQuery {
    /// The query to generate suggestions for.
    q: String,
}

/// A wrapper around `SupportedLanguages`.
#[derive(Debug, PartialEq)]
struct SupportedLanguagesWrapper(SupportedLanguages);

impl FromRequest for SupportedLanguagesWrapper {
    type Config = ();
    type Error = ActixError;
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
        fn parse_language(raw_language: &str) -> Result<Language, ActixError> {
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
    use actix_web::{dev::Payload, http::Method, test::TestRequest, FromRequest, HttpRequest};
    use merino_suggest::{Language, LanguageIdentifier, SupportedLanguages};

    use crate::extractors::SupportedLanguagesWrapper;

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
