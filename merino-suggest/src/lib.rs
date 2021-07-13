#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! Suggestion backends for [Merino](../merino/index.html).

mod multi;
mod wikifruit;

use std::borrow::Cow;

use async_trait::async_trait;
use http::Uri;
use merino_settings::Settings;
use serde::Serialize;
use serde_with::{serde_as, DisplayFromStr};
use thiserror::Error;

pub use crate::multi::Multi;
pub use crate::wikifruit::WikiFruit;

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
#[async_trait]
pub trait SuggestionProvider<'a> {
    /// An operator-visible name for this suggestion provider.
    fn name(&self) -> Cow<'a, str>;

    /// May spawn recurring tasks.
    async fn setup(&mut self, settings: &Settings) -> Result<(), SetupError>;

    /// Provide suggested results for `query`.
    async fn suggest(&self, query: &str) -> Result<Vec<Suggestion>, SuggestError>;
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
pub enum SuggestError {}
