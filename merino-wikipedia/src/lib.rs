//! Tools to process Wikipedia data into a search index, and to create a
//! Suggestion provider that queries that index.

#![warn(missing_docs, clippy::missing_docs_in_private_items)]

mod domain;
mod es_helper;

pub use domain::{WikipediaDocument, WikipediaNamespace};
pub use es_helper::ElasticHelper;
