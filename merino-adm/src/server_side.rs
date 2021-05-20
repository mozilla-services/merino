//! AdM integration that uses adM's server-side API to retrieve suggestions to
//! provide to Firefox.

use http::Uri;
use serde_derive::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

/// Parameters for AdM Conducive API Instant Suggest endpoint, v4.7.21
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub struct SuggestionEndpointParameters {
    /// A code assigned to us by adM.
    partner: String,

    /// The URL encoded partially typed search term (query term). Minimum of 2
    /// characters.
    #[serde(rename = "qt")]
    query_term: String,

    /// The version of the API
    #[serde(rename = "v")]
    api_version: String,

    /// The ISO 3166-1 alpha-2 code of the country the user is in. Example: US
    country_code: String,

    /// The ISO 3166-2 code of the country subdivision the user is in. In the US
    /// this is the level of states. Example: NY
    region_code: String,

    /// The name of the city the user is in. Example: Albany
    city: Option<String>,

    /// The three-digit numeric code for Direct Marketing Area. Only used when
    /// `country_code` is `"US"` and city is specified.
    dma_code: Option<u32>,

    /// The form-factor the user's device
    form_factor: FormFactor,

    /// The family of operating system the user is using.
    os_family: OsFamily,

    /// Maximum number of paid suggestions to return.
    #[serde(rename = "results-ta")]
    max_paid_results: Option<u32>,

    /// Maximum number of organic suggestions to return.
    #[serde(rename = "results-os")]
    max_organic_results: Option<u32>,

    /// Unique identifier for different areas of inventory or ad placements. Only
    /// alphanumeric characters. Maximum 128 characters.
    sub1: String,

    /// Used to further subdivide publisher traffic from a given sub1. Only
    /// alphanumeric characters. Maximum 128 characters.
    sub2: Option<String>,

    /// Used to further subdivide publisher traffic from a given sub2. Only
    /// alphanumeric characters. Maximum 128 characters.
    sub3: Option<String>,

    /// Used to further subdivide publisher traffic from a given sub3. Only
    /// alphanumeric characters. Maximum 128 characters.
    sub4: Option<String>,
}

/// The form factor of a user's device.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[allow(missing_docs, clippy::missing_docs_in_private_items)]
pub enum FormFactor {
    Desktop,
    Phone,
    Tablet,
    Other,
}

/// The operation system a user's device is running.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
#[allow(missing_docs, clippy::missing_docs_in_private_items)]
pub enum OsFamily {
    Windows,
    #[serde(rename = "macOS")]
    Mac,
    Linux,
    #[serde(rename = "iOS")]
    Ios,
    Android,
    #[serde(rename = "ChromeOS")]
    ChromeOs,
    BlackBerry,
    Other,
}

impl From<SuggestionEndpointParameters> for Uri {
    fn from(params: SuggestionEndpointParameters) -> Uri {
        let path_and_query = format!(
            "/suggestionsp?{}",
            serde_qs::to_string(&params).expect("Couldn't make URL params")
        );

        Uri::builder()
            .scheme("https")
            .authority(format!("{}.cpsp.ampfeed.com", params.partner).as_str())
            .path_and_query(path_and_query)
            .build()
            .expect("Couldn't build URL")
    }
}

/// The response from the adM suggestion API.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SuggestionEndpointResponse {
    /// The query that generated these suggestions.
    #[serde(rename = "originalQt")]
    original_query_term: String,

    /// Non-paid suggestions from sources such as Wikipedia.
    organic_suggestions: Vec<Suggestion>,

    /// Paid suggestions from advertisers.
    paid_suggestions: PaidSuggestionsResponse,
}

/// Internal structure of paid suggestion responses.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaidSuggestionsResponse {
    /// This API only includes text results
    text_ads: PaidSuggestionsTextAdsResponse,
}

/// A collection of paid text suggestions.
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PaidSuggestionsTextAdsResponse {
    /// Number of results returned
    results_count: u32,

    /// The suggested ads
    ads: Vec<Suggestion>,
}

/// A suggestion (paid or not) from the adM API.
#[serde_as]
#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Suggestion {
    /// The title of the suggestion
    #[serde(rename = "term")]
    title: String,

    /// The URL to direct the user to on click
    #[serde_as(as = "DisplayFromStr")]
    click_url: Uri,

    /// The URL of the image to show along side the suggestion
    #[serde_as(as = "DisplayFromStr")]
    image_url: Uri,

    /// The URL to notify when the suggestion is displayed
    #[serde_as(as = "DisplayFromStr")]
    impression_url: Uri,

    /// Indicates if visual ad labeling is required to be displayed alongside the suggestion.
    #[serde(rename = "labelRequired")]
    is_ad_label_required: bool,

    /// Indicates if this is a brand ad.
    #[serde(rename = "brand")]
    is_brand_ad: bool,

    /// The brand domain which can be used to autocomplete the user's search term
    brand_domain: Option<String>,

    /// The advertiser URL
    #[serde_as(as = "DisplayFromStr")]
    advertiser_url: Uri,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn conducive_parameter_example() {
        let params = SuggestionEndpointParameters {
            partner: "test_partner".into(),
            query_term: "am".into(),
            api_version: "1.0".into(),
            country_code: "US".into(),
            region_code: "NY".into(),
            city: Some("Albany".into()),
            dma_code: Some(532),
            form_factor: FormFactor::Desktop,
            os_family: OsFamily::Mac,
            max_paid_results: None,
            max_organic_results: None,
            sub1: "level1".into(),
            sub2: Some("level2".into()),
            sub3: Some("level3".into()),
            sub4: Some("level4".into()),
        };
        assert_eq!(
            Into::<Uri>::into(params),
            Uri::from_maybe_shared(concat!(
                "https://test_partner.cpsp.ampfeed.com/suggestionsp",
                "?partner=test_partner",
                "&qt=am",
                "&v=1.0",
                "&country-code=US",
                "&region-code=NY",
                "&city=Albany",
                "&dma-code=532",
                "&form-factor=desktop",
                "&os-family=macOS",
                "&sub1=level1",
                "&sub2=level2",
                "&sub3=level3",
                "&sub4=level4",
            ))
            .expect("bad test data")
        );
    }

    #[test]
    fn conducive_response_example() {
        let json = r#"
            {
                "originalQt": "am",
                "organicSuggestions": [],
                "paidSuggestions": {
                    "textAds": {
                        "ads": [
                            {
                                "term": "amazon.com - Huge Selection & Amazing Prices",
                                "clickUrl": "https://bridge.lga1.admarketplace.net/ctp?version=1...",
                                "imageUrl": "https://cdn.45tu1c0.com/account/74042/200/152122808...",
                                "impressionUrl": "https://imp.mt48.net/imp?id=7R7wx...",
                                "labelRequired": false,
                                "brand": true,
                                "brandDomain": "amazon.com",
                                "advertiserUrl": "https://www.amazon.com/?tag=admarketus-20&ref=p..."
                            }
                        ],
                        "resultsCount": 1
                    }
                }
            }
        "#;
        let actual: SuggestionEndpointResponse =
            serde_json::from_str(&json).expect("Could not parse test data");
        assert_eq!(
            actual,
            SuggestionEndpointResponse {
                organic_suggestions: vec![],
                original_query_term: "am".into(),
                paid_suggestions: PaidSuggestionsResponse {
                    text_ads: PaidSuggestionsTextAdsResponse {
                        results_count: 1,
                        ads: vec![Suggestion {
                            title: "amazon.com - Huge Selection & Amazing Prices".into(),
                            click_url: Uri::from_static(
                                "https://bridge.lga1.admarketplace.net/ctp?version=1..."
                            ),
                            image_url: Uri::from_static(
                                "https://cdn.45tu1c0.com/account/74042/200/152122808..."
                            ),
                            impression_url: Uri::from_static(
                                "https://imp.mt48.net/imp?id=7R7wx..."
                            ),
                            is_ad_label_required: false,
                            is_brand_ad: true,
                            brand_domain: Some("amazon.com".into()),
                            advertiser_url: Uri::from_static(
                                "https://www.amazon.com/?tag=admarketus-20&ref=p..."
                            ),
                        }],
                    }
                }
            }
        )
    }
}
