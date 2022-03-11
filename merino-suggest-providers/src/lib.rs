#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! All of the merino suggestion providers, including tools to make and combine them.

mod maker;
mod providers;
mod reconfigure;

pub use crate::{
    maker::make_provider_tree,
    providers::{
        client_variant_filter::ClientVariantFilterProvider,
        debug::DebugProvider,
        fixed::FixedProvider,
        id_multi::{IdMulti, ProviderDetails as IdMultiProviderDetails},
        keyword_filter::KeywordFilterProvider,
        live_query_demo::LiveQuerySuggester,
        multi::Multi,
        stealth::StealthProvider,
        timeout::TimeoutProvider,
        wikifruit::WikiFruit,
    },
    reconfigure::reconfigure_provider_tree,
};
pub use merino_suggest_traits::NullProvider;
