#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! Suggestion backends for [Merino](../merino/index.html).

use serde::Serialize;

/// A suggestion to provide to a user.
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
    pub url: String,

    /// The URL to notify when this keyword is presented to a user.
    pub impression_url: String,

    /// The name of the advertiser associated with this suggestion.
    pub advertiser: String,

    /// Whether this suggestion is sponsored.
    pub is_sponsored: bool,

    /// The URL of the icon to show along side this suggestion.
    pub icon: String,
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
                url: "https://en.wikipedia.org/wiki/Apple".to_string(),
                impression_url: "https://127.0.0.1/".to_string(),
                advertiser: "Merino::WikiFruit".to_string(),
                is_sponsored: false,
                icon: "https://en.wikipedia.org/favicon.ico".to_string(),
            }),
            "banana" => Some(Suggestion {
                id: 1,
                full_keyword: "banana".to_string(),
                title: "Wikipedia - Banana".to_string(),
                url: "https://en.wikipedia.org/wiki/Banana".to_string(),
                impression_url: "https://127.0.0.1/".to_string(),
                advertiser: "Merino::WikiFruit".to_string(),
                is_sponsored: false,
                icon: "https://en.wikipedia.org/favicon.ico".to_string(),
            }),
            "cherry" => Some(Suggestion {
                id: 1,
                full_keyword: "cherry".to_string(),
                title: "Wikipedia - Cherry".to_string(),
                url: "https://en.wikipedia.org/wiki/Cherry".to_string(),
                impression_url: "https://127.0.0.1/".to_string(),
                advertiser: "Merino::WikiFruit".to_string(),
                is_sponsored: false,
                icon: "https://en.wikipedia.org/favicon.ico".to_string(),
            }),
            _ => None,
        };

        suggestion.into_iter().collect()
    }
}
