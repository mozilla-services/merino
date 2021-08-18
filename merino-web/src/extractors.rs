//! Types to extract merino data from requests.

use std::borrow::Cow;
use std::str::FromStr;

use crate::errors::HandlerError;
use actix_web::{
    dev::Payload,
    http::{header, HeaderValue},
    web::Query,
    Error as ActixError, FromRequest, HttpRequest,
};
use actix_web_location::Location;
use futures_util::{
    future::{self, LocalBoxFuture, Ready},
    FutureExt,
};
use lazy_static::lazy_static;
use merino_suggest::{
    DeviceInfo, FormFactor, Language, LanguageIdentifier, OsFamily, SuggestionRequest,
    SupportedLanguages,
};
use serde::Deserialize;
use tokio::try_join;
use woothee::parser::Parser;

lazy_static! {
    static ref EMPTY_HEADER: HeaderValue = HeaderValue::from_static("");
}

/// An extractor for a [`merino_suggest::SuggestionRequest`].
pub struct SuggestionRequestWrapper<'a>(pub SuggestionRequest<'a>);

impl<'a> FromRequest for SuggestionRequestWrapper<'a> {
    type Config = ();

    type Error = ActixError;

    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        async move {
            /// try_join wants everything to have the same error type, and
            /// doesn't give a chance to map the error. This wrapper helps that.
            async fn loc_mapped_error(request: &HttpRequest) -> Result<Location, ActixError> {
                Location::extract(request).await.map_err(ActixError::from)
            }

            // Retrieve all parts needed to make a SuggestionRequest concurrently.
            // `try_join` implicitly `.await`s.
            let (
                Query(SuggestQuery { q: query }),
                SupportedLanguagesWrapper(supported_languages),
                location,
                DeviceInfoWrapper(device_info),
            ) = try_join!(
                Query::extract(&req),
                SupportedLanguagesWrapper::extract(&req),
                loc_mapped_error(&req),
                DeviceInfoWrapper::extract(&req),
            )?;

            Ok(Self(SuggestionRequest {
                query: query.into(),
                accepts_english: supported_languages.includes("en", None),
                country: location.country.map(Cow::from),
                region: location.region.map(Cow::from),
                dma: location.dma,
                city: location.city.map(Cow::from),
                device_info,
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

/// A wrapper around [`SupportedLanguages`].
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
            let header = match req.headers().get(header::ACCEPT_LANGUAGE) {
                Some(header) => header.to_str().map_err::<Self::Error, _>(|_| {
                    HandlerError::MalformedHeader("Accept-Language").into()
                }),
                None => return Ok(Self(SupportedLanguages::wildcard())),
            }?;

            let languages = header
                .split(',')
                .map(str::trim)
                .map(parse_language)
                .collect::<Result<Vec<Language>, _>>()?;

            Ok(Self(SupportedLanguages(languages)))
        };

        future::ready(parse_header())
    }
}

/// A wrapper around [`DeviceInfo`].
#[derive(Debug, Default, PartialEq)]
struct DeviceInfoWrapper(DeviceInfo);

impl FromRequest for DeviceInfoWrapper {
    type Config = ();
    type Error = ActixError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let header = req
            .headers()
            .get(header::USER_AGENT)
            .unwrap_or(&EMPTY_HEADER)
            .to_str()
            .unwrap_or_default();

        if header.is_empty() {
            return future::ready(Ok(DeviceInfoWrapper::default()));
        }

        let wresult = Parser::new().parse(header).unwrap_or_default();

        // If it's not firefox, it doesn't belong here...
        if !["firefox"].contains(&wresult.name.to_lowercase().as_str()) {
            return future::ready(Err(HandlerError::InvalidHeader(
                "User agent is not a Firefox user agent",
            )
            .into()));
        }

        let os = wresult.os.to_lowercase();
        let os_family = match os.as_str() {
            _ if os.starts_with("windows") => OsFamily::Windows,
            "mac osx" => OsFamily::MacOs,
            "linux" => OsFamily::Linux,
            "iphone" | "ipad" => OsFamily::IOs,
            "android" => OsFamily::Android,
            "chromeos" => OsFamily::ChromeOs,
            "blackberry" => OsFamily::BlackBerry,
            _ => OsFamily::Other,
        };

        let form_factor = match wresult.category {
            "pc" => FormFactor::Desktop,
            "smartphone" if os.as_str() == "ipad" => FormFactor::Tablet,
            "smartphone" => FormFactor::Phone,
            _ => FormFactor::Other,
        };

        let ff_version = Some(
            u32::from_str(wresult.version.split('.').collect::<Vec<&str>>()[0]).unwrap_or_default(),
        );

        future::ready(Ok(DeviceInfoWrapper(DeviceInfo {
            os_family,
            form_factor,
            ff_version,
        })))
    }
}

#[cfg(test)]
mod tests {
    use actix_web::{dev::Payload, http::Method, test::TestRequest, FromRequest, HttpRequest};
    use merino_suggest::{
        DeviceInfo, FormFactor, Language, LanguageIdentifier, OsFamily, SupportedLanguages,
    };
    use pretty_assertions::assert_eq;

    use crate::extractors::{DeviceInfoWrapper, SupportedLanguagesWrapper};

    const SUGGEST_URI: &str = "/api/v1/suggest";

    fn test_request_with_header(header: (&'static str, &'static str)) -> HttpRequest {
        TestRequest::with_uri(SUGGEST_URI)
            .insert_header(header)
            .method(Method::GET)
            .param("q", "asdf")
            .to_http_request()
    }

    #[actix_rt::test]
    async fn test_valid_accept_language_headers() {
        let mut payload = Payload::None;

        // Test single language without region
        let req = test_request_with_header(("Accept-Language", "en"));
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
        let req = test_request_with_header(("Accept-Language", "en-US"));
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
        let req = test_request_with_header(("Accept-Language", "*"));
        let result = SupportedLanguagesWrapper::from_request(&req, &mut payload)
            .await
            .expect("Could not get result in test_valid_accept_language_headers");
        let expected_supported_languages_wrapper =
            SupportedLanguagesWrapper(SupportedLanguages::wildcard());

        assert_eq!(expected_supported_languages_wrapper, result);

        // Test several languages with quality values
        let req = test_request_with_header((
            "Accept-Language",
            "fr-CH, fr;q=0.9, en;q=0.8, de;q=0.7, *;q=0.5",
        ));
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
        let req = test_request_with_header(("Accept-Language", "en-US;3"));
        let result = SupportedLanguagesWrapper::from_request(&req, &mut payload).await;

        assert_eq!(
            "Malformed header: Accept-Language",
            result.unwrap_err().to_string()
        );

        // Header with non-visible ASCII characters (\u{200B} is the zero-width space character)
        let req = test_request_with_header(("Accept-Language", "\u{200B}"));
        let result = SupportedLanguagesWrapper::from_request(&req, &mut payload).await;

        assert_eq!(
            "Malformed header: Accept-Language",
            result.unwrap_err().to_string()
        );

        // Non-numeric quality value
        let req = test_request_with_header(("Accept-Language", "en-US;q=one"));
        let result = SupportedLanguagesWrapper::from_request(&req, &mut payload).await;

        assert_eq!(
            "Malformed header: Accept-Language",
            result.unwrap_err().to_string()
        );
    }

    #[actix_rt::test]
    async fn test_valid_non_english_language_headers() {
        let mut payload = Payload::None;
        let req = test_request_with_header(("Accept-Language", "es-ES;q=1.0,es-MX;q=0.5,es;q=0.7"));
        let supported_languages = SupportedLanguagesWrapper::from_request(&req, &mut payload)
            .await
            .unwrap()
            .0;

        let expected_supported_languages = SupportedLanguages(vec![
            Language::locale("es", Some("es"), Some(1.0)),
            Language::locale("es", Some("mx"), Some(0.5)),
            Language::locale::<_, String>("es", None, Some(0.7)),
        ]);

        assert_eq!(supported_languages, expected_supported_languages);
        assert!(!supported_languages.includes("en", None));
    }

    #[actix_rt::test]
    async fn test_valid_user_agents() {
        // macOS
        let header = (
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 11.2; rv:85.0) Gecko/20100101 Firefox/85.0",
        );
        assert_eq!(
            DeviceInfoWrapper::extract(&test_request_with_header(header))
                .await
                .expect("Count not get result in test_valid_user_agents"),
            DeviceInfoWrapper(DeviceInfo {
                os_family: OsFamily::MacOs,
                form_factor: FormFactor::Desktop,
                ff_version: Some(85),
            })
        );

        // Windows
        let header = (
            "User-Agent",
            "Mozilla/5.0 (Windows NT 6.1; Win64; x64; rv:61.0) Gecko/20100101 Firefox/61.0",
        );
        assert_eq!(
            DeviceInfoWrapper::extract(&test_request_with_header(header))
                .await
                .expect("Count not get result in test_valid_user_agents"),
            DeviceInfoWrapper(DeviceInfo {
                os_family: OsFamily::Windows,
                form_factor: FormFactor::Desktop,
                ff_version: Some(61),
            })
        );

        // Linux
        let header = (
            "User-Agent",
            "Mozilla/5.0 (X11; Fedora; Linux x86_64; rv:82.0.1) Gecko/20100101 Firefox/82.0.1",
        );
        assert_eq!(
            DeviceInfoWrapper::extract(&test_request_with_header(header))
                .await
                .expect("Count not get result in test_valid_user_agents"),
            DeviceInfoWrapper(DeviceInfo {
                os_family: OsFamily::Linux,
                form_factor: FormFactor::Desktop,
                ff_version: Some(82),
            })
        );

        // Android
        let header = (
            "User-Agent",
            "Mozilla/5.0 (Android 11; Mobile; rv:68.0) Gecko/68.0 Firefox/85.0",
        );
        assert_eq!(
            DeviceInfoWrapper::extract(&test_request_with_header(header))
                .await
                .expect("Count not get result in test_valid_user_agents"),
            DeviceInfoWrapper(DeviceInfo {
                os_family: OsFamily::Android,
                form_factor: FormFactor::Phone,
                ff_version: Some(85),
            })
        );

        // iOS (iPhone)
        let header = ("User-Agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 8_3 like Mac OS X) AppleWebKit/600.1.4 (KHTML, like Gecko) FxiOS/2.0 Mobile/12F69 Safari/600.1.4");
        assert_eq!(
            DeviceInfoWrapper::extract(&test_request_with_header(header))
                .await
                .expect("Count not get result in test_valid_user_agents"),
            DeviceInfoWrapper(DeviceInfo {
                os_family: OsFamily::IOs,
                form_factor: FormFactor::Phone,
                ff_version: Some(2),
            })
        );

        // iOS (iPad)
        let header = ("User-Agent", "Mozilla/5.0 (iPad; CPU iPhone OS 8_3 like Mac OS X) AppleWebKit/600.1.4 (KHTML, like Gecko) FxiOS/1.0 Mobile/12F69 Safari/600.1.4");
        assert_eq!(
            DeviceInfoWrapper::extract(&test_request_with_header(header))
                .await
                .expect("Count not get result in test_valid_user_agents"),
            DeviceInfoWrapper(DeviceInfo {
                os_family: OsFamily::IOs,
                form_factor: FormFactor::Tablet,
                ff_version: Some(1),
            })
        );

        // No user agent header
        let request = TestRequest::with_uri(SUGGEST_URI)
            .method(Method::GET)
            .param("q", "asdf")
            .to_http_request();
        assert_eq!(
            DeviceInfoWrapper::extract(&request)
                .await
                .expect("Count not get result in test_valid_user_agents"),
            DeviceInfoWrapper(DeviceInfo {
                os_family: OsFamily::Other,
                form_factor: FormFactor::Other,
                ff_version: None,
            })
        );
    }

    #[actix_rt::test]
    async fn test_invalid_user_agents() {
        // Not a Firefox user agent
        let header = ("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 11_2) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/88.0.4324.150 Safari/537.36");
        assert_eq!(
            DeviceInfoWrapper::extract(&test_request_with_header(header))
                .await
                .unwrap_err()
                .to_string(),
            "Invalid header: User agent is not a Firefox user agent"
        );
    }
}
