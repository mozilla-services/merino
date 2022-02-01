#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! All of the merino suggestion providers, including tools to make and combine them.

mod maker;
mod providers;

pub use crate::{
    maker::make_provider_tree,
    providers::{
        client_variant_filter::ClientVariantFilterProvider,
        debug::DebugProvider,
        fixed::FixedProvider,
        id_multi::{IdMulti, ProviderDetails as IdMultiProviderDetails},
        keyword_filter::KeywordFilterProvider,
        multi::Multi,
        stealth::StealthProvider,
        timeout::TimeoutProvider,
        wikifruit::WikiFruit,
    },
};
pub use merino_suggest_traits::NullProvider;
