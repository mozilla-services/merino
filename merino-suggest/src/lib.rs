#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! Suggestion backends for [Merino](../merino/index.html).

use serde::Serialize;

/// A suggestion to provide to a user.
#[derive(Clone, Debug, Serialize, PartialEq)]
pub struct Suggestion {
    /// The URL to send the user to if they select this suggestion.
    pub url: String,
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
        let url = match query {
            "apple" => Some("https://en.wikipedia.org/wiki/Apple"),
            "banana" => Some("https://en.wikipedia.org/wiki/Banana"),
            "cherry" => Some("https://en.wikipedia.org/wiki/Cherry"),
            _ => None,
        };
        if let Some(url) = url {
            vec![Suggestion {
                url: url.to_string(),
            }]
        } else {
            vec![]
        }
    }
}
