//! Types to extract merino data from requests.

use std::str::FromStr;

use actix_web::{
    dev::Payload,
    http::header::{self, AcceptLanguage, Header, HeaderValue, LanguageTag},
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
    device_info::{Browser, DeviceInfo, FormFactor, OsFamily},
    SuggestionRequest, SupportedLanguages,
};
use serde::Deserialize;
use serde_with::{rust::StringWithSeparator, serde_as, CommaSeparator};
use tokio::try_join;
use woothee::parser::{Parser, WootheeResult};

lazy_static! {
    static ref EMPTY_HEADER: HeaderValue = HeaderValue::from_static("");
}

/// An extractor for a [`merino_suggest::SuggestionRequest`].
pub struct SuggestionRequestWrapper(pub SuggestionRequest);

impl FromRequest for SuggestionRequestWrapper {
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
                Query(SuggestQuery {
                    q: query,
                    client_variants,
                }),
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
                query,
                accepts_english: supported_languages.includes(LanguageTag::parse("en").unwrap()),
                country: location.country,
                region: location.region,
                dma: location.dma,
                city: location.city,
                device_info,
                client_variants,
            }))
        }
        .boxed_local()
    }
}

/// A query passed to the API.
#[serde_as]
#[derive(Debug, Deserialize)]
struct SuggestQuery {
    /// The query to generate suggestions for.
    q: String,
    /// The client variants to generate suggestions with.
    #[serde_as(as = "StringWithSeparator::<CommaSeparator, String>")]
    client_variants: Vec<String>,
}

/// A wrapper around [`SupportedLanguages`].
#[derive(Debug, PartialEq)]
struct SupportedLanguagesWrapper(SupportedLanguages);

impl FromRequest for SupportedLanguagesWrapper {
    type Error = ActixError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        future::ready({
            if req.headers().contains_key("Accept-Language") {
                match AcceptLanguage::parse(req) {
                    // AcceptLanguage::parse() returns an empty Vec for certain types of
                    // errors in the header. In these cases, we assume that the client will accept
                    // any language.
                    Ok(languages) if languages.is_empty() => {
                        Ok(Self(SupportedLanguages::wildcard()))
                    }
                    // If an error occurs while parsing the header, we assume that the client will
                    // accept any language.
                    Err(_) => Ok(Self(SupportedLanguages::wildcard())),
                    Ok(languages) => Ok(Self(SupportedLanguages(languages))),
                }
            } else {
                // If the request does not have an Accept-Language header at all, we assume that
                // the client will accept any language.
                Ok(Self(SupportedLanguages::wildcard()))
            }
        })
    }
}

/// A wrapper around [`DeviceInfo`].
#[derive(Debug, PartialEq)]
struct DeviceInfoWrapper(DeviceInfo);

impl FromRequest for DeviceInfoWrapper {
    type Error = ActixError;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let header = req
            .headers()
            .get(header::USER_AGENT)
            .unwrap_or(&EMPTY_HEADER)
            .to_str()
            .unwrap_or_default();
        let wresult = Parser::new().parse(header).unwrap_or_default();

        future::ready(Ok(DeviceInfoWrapper::from_woothee_result(&wresult)))
    }
}

/// Extracts information from a [`WootheeResult`].
trait FromWootheeResult {
    /// Extracts information from a [`WootheeResult`].
    fn from_woothee_result(wresult: &WootheeResult) -> Self;
}

impl FromWootheeResult for DeviceInfoWrapper {
    fn from_woothee_result(wresult: &WootheeResult) -> Self {
        Self(DeviceInfo::from_woothee_result(wresult))
    }
}

impl FromWootheeResult for DeviceInfo {
    fn from_woothee_result(wresult: &WootheeResult) -> Self {
        Self {
            os_family: OsFamily::from_woothee_result(wresult),
            form_factor: FormFactor::from_woothee_result(wresult),
            browser: Browser::from_woothee_result(wresult),
        }
    }
}

impl FromWootheeResult for FormFactor {
    fn from_woothee_result(wresult: &WootheeResult) -> Self {
        let os = wresult.os.to_lowercase();

        match wresult.category {
            "pc" => FormFactor::Desktop,
            "smartphone" if os == "ipad" => FormFactor::Tablet,
            "smartphone" => FormFactor::Phone,
            _ => FormFactor::Other,
        }
    }
}

impl FromWootheeResult for OsFamily {
    fn from_woothee_result(wresult: &WootheeResult) -> Self {
        let os = wresult.os.to_lowercase();

        match os.as_str() {
            _ if os.starts_with("windows") => OsFamily::Windows,
            "mac osx" => OsFamily::MacOs,
            "linux" => OsFamily::Linux,
            "iphone" | "ipad" => OsFamily::IOs,
            "android" => OsFamily::Android,
            "chromeos" => OsFamily::ChromeOs,
            "blackberry" => OsFamily::BlackBerry,
            _ => OsFamily::Other,
        }
    }
}

impl FromWootheeResult for Browser {
    fn from_woothee_result(wresult: &WootheeResult) -> Self {
        if wresult.name.to_lowercase() == "firefox" {
            let version = u32::from_str(wresult.version.split('.').collect::<Vec<&str>>()[0])
                .unwrap_or_default();
            Browser::Firefox(version)
        } else {
            Browser::Other
        }
    }
}

#[cfg(test)]
mod tests {
    use actix_web::{
        dev::Payload,
        http::{
            header::{q, AcceptLanguage, LanguageTag, Preference, QualityItem},
            Method,
        },
        test::TestRequest,
        FromRequest, HttpRequest,
    };
    use merino_suggest::{
        device_info::{Browser, DeviceInfo, FormFactor, OsFamily},
        SupportedLanguages,
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

        let req = test_request_with_header((
            "Accept-Language",
            "fr-CH, fr;q=0.9, en;q=0.8, de;q=0.7, *;q=0.5",
        ));
        let result = SupportedLanguagesWrapper::from_request(&req, &mut payload)
            .await
            .expect("Could not get result in test_valid_accept_language_headers");
        let expected_supported_languages_wrapper =
            SupportedLanguagesWrapper(SupportedLanguages(AcceptLanguage(vec![
                QualityItem::max(Preference::Specific(LanguageTag::parse("fr-CH").unwrap())),
                QualityItem {
                    item: Preference::Specific(LanguageTag::parse("fr").unwrap()),
                    quality: q(0.9),
                },
                QualityItem {
                    item: Preference::Specific(LanguageTag::parse("en").unwrap()),
                    quality: q(0.8),
                },
                QualityItem {
                    item: Preference::Specific(LanguageTag::parse("de").unwrap()),
                    quality: q(0.7),
                },
                QualityItem {
                    item: Preference::Any,
                    quality: q(0.5),
                },
            ])));

        assert_eq!(expected_supported_languages_wrapper, result);
    }

    #[actix_rt::test]
    async fn test_invalid_accept_language_headers() {
        let mut payload = Payload::None;
        let expected_supported_languages_wrapper =
            SupportedLanguagesWrapper(SupportedLanguages::wildcard());

        // Malformed quality value
        let req = test_request_with_header(("Accept-Language", "en-US;3"));
        let result = SupportedLanguagesWrapper::from_request(&req, &mut payload)
            .await
            .expect("Could not get result in test_invalid_accept_language_headers");

        assert_eq!(expected_supported_languages_wrapper, result);

        // Header with non-visible ASCII characters (\u{200B} is the zero-width space character)
        let req = test_request_with_header(("Accept-Language", "\u{200B}"));
        let result = SupportedLanguagesWrapper::from_request(&req, &mut payload)
            .await
            .expect("Could not get result in test_invalid_accept_language_headers");

        assert_eq!(expected_supported_languages_wrapper, result);

        // Non-numeric quality value
        let req = test_request_with_header(("Accept-Language", "en-US;q=one"));
        let result = SupportedLanguagesWrapper::from_request(&req, &mut payload)
            .await
            .expect("Could not get result in test_invalid_accept_language_headers");

        assert_eq!(expected_supported_languages_wrapper, result);
    }

    #[actix_rt::test]
    async fn test_valid_non_english_language_headers() {
        let mut payload = Payload::None;
        let req = test_request_with_header(("Accept-Language", "es-ES;q=1.0,es-MX;q=0.5,es;q=0.7"));
        let supported_languages = SupportedLanguagesWrapper::from_request(&req, &mut payload)
            .await
            .unwrap()
            .0;

        let expected_supported_languages = SupportedLanguages(AcceptLanguage(vec![
            QualityItem::max(Preference::Specific(LanguageTag::parse("es-ES").unwrap())),
            QualityItem {
                item: Preference::Specific(LanguageTag::parse("es-MX").unwrap()),
                quality: q(0.5),
            },
            QualityItem {
                item: Preference::Specific(LanguageTag::parse("es").unwrap()),
                quality: q(0.7),
            },
        ]));

        assert_eq!(supported_languages, expected_supported_languages);
        assert!(!supported_languages.includes(LanguageTag::parse("en").unwrap()));
    }

    #[actix_rt::test]
    async fn test_wildcard_language_headers() {
        let mut payload = Payload::None;
        let req = test_request_with_header(("Accept-Language", "*"));
        let supported_languages = SupportedLanguagesWrapper::from_request(&req, &mut payload)
            .await
            .unwrap()
            .0;
        let expected_supported_languages =
            SupportedLanguages(AcceptLanguage(vec![QualityItem::max(Preference::Any)]));

        assert_eq!(supported_languages, expected_supported_languages);
    }

    #[actix_rt::test]
    async fn test_request_with_no_language_headers() {
        let mut payload = Payload::None;
        let req = TestRequest::with_uri(SUGGEST_URI)
            .method(Method::GET)
            .param("q", "asdf")
            .to_http_request();
        let supported_languages = SupportedLanguagesWrapper::from_request(&req, &mut payload)
            .await
            .unwrap()
            .0;
        let expected_supported_languages =
            SupportedLanguages(AcceptLanguage(vec![QualityItem::max(Preference::Any)]));

        assert_eq!(supported_languages, expected_supported_languages);
    }

    #[actix_rt::test]
    async fn test_macos_user_agent() {
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
                browser: Browser::Firefox(85),
            })
        );
    }

    #[actix_rt::test]
    async fn test_windows_user_agent() {
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
                browser: Browser::Firefox(61),
            })
        );
    }

    #[actix_rt::test]
    async fn test_linux_user_agent() {
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
                browser: Browser::Firefox(82),
            })
        );
    }

    #[actix_rt::test]
    async fn test_android_user_agent() {
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
                browser: Browser::Firefox(85),
            })
        );
    }

    #[actix_rt::test]
    async fn test_iphone_user_agent() {
        let header = ("User-Agent", "Mozilla/5.0 (iPhone; CPU iPhone OS 8_3 like Mac OS X) AppleWebKit/600.1.4 (KHTML, like Gecko) FxiOS/2.0 Mobile/12F69 Safari/600.1.4");
        assert_eq!(
            DeviceInfoWrapper::extract(&test_request_with_header(header))
                .await
                .expect("Count not get result in test_valid_user_agents"),
            DeviceInfoWrapper(DeviceInfo {
                os_family: OsFamily::IOs,
                form_factor: FormFactor::Phone,
                browser: Browser::Firefox(2),
            })
        );
    }

    #[actix_rt::test]
    async fn test_ipad_user_agent() {
        let header = ("User-Agent", "Mozilla/5.0 (iPad; CPU iPhone OS 8_3 like Mac OS X) AppleWebKit/600.1.4 (KHTML, like Gecko) FxiOS/1.0 Mobile/12F69 Safari/600.1.4");
        assert_eq!(
            DeviceInfoWrapper::extract(&test_request_with_header(header))
                .await
                .expect("Count not get result in test_valid_user_agents"),
            DeviceInfoWrapper(DeviceInfo {
                os_family: OsFamily::IOs,
                form_factor: FormFactor::Tablet,
                browser: Browser::Firefox(1),
            })
        );
    }

    #[actix_rt::test]
    async fn test_missing_user_agent() {
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
                browser: Browser::Other,
            })
        );
    }

    #[actix_rt::test]
    async fn test_non_firefox_user_agent() {
        let header = (
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/74.0.3729.169 Safari/537.36",
        );
        assert_eq!(
            DeviceInfoWrapper::extract(&test_request_with_header(header))
                .await
                .expect("Count not get result in test_valid_user_agents"),
            DeviceInfoWrapper(DeviceInfo {
                os_family: OsFamily::Windows,
                form_factor: FormFactor::Desktop,
                browser: Browser::Other,
            })
        );
    }
}
