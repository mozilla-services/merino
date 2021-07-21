#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! Manages Merino's cache

mod domain;
mod redis;

pub use crate::redis::Suggester as RedisSuggester;
