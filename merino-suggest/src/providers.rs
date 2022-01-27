//! Merino providers that aren't tied to any particular data source.

mod client_variant_filter;
mod debug;
mod fixed;
mod id_multi;
mod keyword_filter;
mod multi;
mod stealth;
mod timeout;
mod wikifruit;

pub use self::client_variant_filter::ClientVariantFilterProvider;
pub use self::debug::DebugProvider;
pub use self::fixed::FixedProvider;
pub use self::id_multi::{IdMulti, ProviderDetails as IdMultiProviderDetails};
pub use self::keyword_filter::KeywordFilterProvider;
pub use self::multi::Multi;
pub use self::stealth::StealthProvider;
pub use self::timeout::TimeoutProvider;
pub use self::wikifruit::WikiFruit;
