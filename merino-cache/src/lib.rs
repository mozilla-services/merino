#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! Manages Merino's cache

#[macro_use]
extern crate lazy_static;

mod deduped_map;
mod domain;
mod memory;
mod redis;

pub use crate::memory::Suggester as MemoryCacheSuggester;
pub use crate::redis::Suggester as RedisCacheSuggester;
