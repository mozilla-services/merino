use actix_web::http::Uri;
use serde_derive::{Deserialize, Serialize};
use std::net::Ipv4Addr;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct SuggestionEndpointParameters {
    /// The partner name assigned by adMarketplace.
    partner: String,

    /// The URL encoded partially typed search term (query term). Minimum of 2
    /// characters.
    #[serde(rename = "qt")]
    query_term: String,

    /// Originally, the IP address of the user making the request. In our
    /// implementation, one of regional IP addresses we make requests from to
    /// mask client IPs.
    ip: Ipv4Addr,

    /// Originally, the user agent of the user's browser. In our implementation,
    /// a normalized version of it to protect privacy.
    #[serde(rename = "ua")]
    user_agent: String,

    /// The version of the API
    #[serde(rename = "v")]
    api_version: String,

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

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct SuggestionEndpointResponse {
    #[serde(rename = "originalQt")]
    original_query_term: String,

    organic_suggestions: Vec<Suggestion>,

    paid_suggestions: PaidSuggestionsResponse,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct PaidSuggestionsResponse {
    text_ads: PaidSuggestionsTextAdsResponse,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct PaidSuggestionsTextAdsResponse {
    /// Number of results returned
    results_count: u32,

    /// The suggested ads
    ads: Vec<Suggestion>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Suggestion {
    /// The title of the suggestion
    #[serde(rename = "term")]
    title: String,

    /// The URL to direct the user to on click
    click_url: String,

    /// The image to show along side the suggestion
    image_url: String,

    /// The URL to notify when the suggestion is displayed
    impression_url: String,

    /// Indicates if visual ad labeling is required to be displayed alongside the suggestion.
    #[serde(rename = "labelRequired")]
    is_ad_label_required: bool,

    /// Indicates if this is a brand ad.
    #[serde(rename = "brand")]
    is_brand_ad: bool,

    /// The brand domain which can be used to autocomplete the user's search term
    brand_domain: Option<String>,

    /// The advertiser URL
    advertiser_url: String,
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
            ip: Ipv4Addr::new(130, 245, 32, 23),
            user_agent: "Mozilla/5.0 (X11; Linux x86_64; rv:89.0) Gecko/20100101 Firefox/89.0"
                .into(),
            api_version: "1.0".into(),
            max_paid_results: None,
            max_organic_results: None,
            sub1: "quick_suggest".into(),
            sub2: None,
            sub3: None,
            sub4: None,
        };
        assert_eq!(
            Into::<Uri>::into(params),
            Uri::from_maybe_shared("https://test_partner.cpsp.ampfeed.com/suggestionsp?partner=test_partner&qt=am&ip=130.245.32.23&ua=Mozilla%2F5.0+%28X11%3B+Linux+x86_64%3B+rv%3A89.0%29+Gecko%2F20100101+Firefox%2F89.0&v=1.0&sub1=quick_suggest")
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
                            click_url: "https://bridge.lga1.admarketplace.net/ctp?version=1..."
                                .into(),
                            image_url: "https://cdn.45tu1c0.com/account/74042/200/152122808..."
                                .into(),
                            impression_url: "https://imp.mt48.net/imp?id=7R7wx...".into(),
                            is_ad_label_required: false,
                            is_brand_ad: true,
                            brand_domain: Some("amazon.com".into()),
                            advertiser_url: "https://www.amazon.com/?tag=admarketus-20&ref=p..."
                                .into(),
                        }],
                    }
                }
            }
        )
    }
}
