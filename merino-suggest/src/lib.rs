#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! Suggestion backends for [Merino](../merino/index.html).

use http::Uri;
use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};

/// A suggestion to provide to a user.
#[serde_as]
#[derive(Clone, Debug, Serialize, PartialEq)]
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
pub trait Suggester {
    /// Provide suggested results for `query`.
    fn suggest(&self, query: &str) -> Vec<Suggestion>;
}

/// A toy suggester to test the system.
pub struct WikiFruit;

impl Suggester for WikiFruit {
    fn suggest(&self, query: &str) -> Vec<Suggestion> {
        let suggestion = match query {
            "apple" => Some(Suggestion {
                id: 1,
                full_keyword: "apple".to_string(),
                title: "Wikipedia - Apple".to_string(),
                url: Uri::from_static("https://en.wikipedia.org/wiki/Apple"),
                impression_url: Uri::from_static("https://127.0.0.1/"),
                click_url: Uri::from_static("https://127.0.0.1/"),
                advertiser: "Merino::WikiFruit".to_string(),
                is_sponsored: false,
                icon: Uri::from_static("https://en.wikipedia.org/favicon.ico"),
            }),
            "banana" => Some(Suggestion {
                id: 1,
                full_keyword: "banana".to_string(),
                title: "Wikipedia - Banana".to_string(),
                url: Uri::from_static("https://en.wikipedia.org/wiki/Banana"),
                impression_url: Uri::from_static("https://127.0.0.1/"),
                click_url: Uri::from_static("https://127.0.0.1/"),
                advertiser: "Merino::WikiFruit".to_string(),
                is_sponsored: false,
                icon: Uri::from_static("https://en.wikipedia.org/favicon.ico"),
            }),
            "cherry" => Some(Suggestion {
                id: 1,
                full_keyword: "cherry".to_string(),
                title: "Wikipedia - Cherry".to_string(),
                url: Uri::from_static("https://en.wikipedia.org/wiki/Cherry"),
                impression_url: Uri::from_static("https://127.0.0.1/"),
                click_url: Uri::from_static("https://127.0.0.1/"),
                advertiser: "Merino::WikiFruit".to_string(),
                is_sponsored: false,
                icon: Uri::from_static("https://en.wikipedia.org/favicon.ico"),
            }),
            _ => None,
        };

        suggestion.into_iter().collect()
    }
}
