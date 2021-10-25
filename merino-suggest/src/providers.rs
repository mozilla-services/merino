//! Merino providers that aren't tied to any particular data source.

mod debug;
mod fixed;
mod id_multi;
mod keyword_filter;
mod multi;
mod timeout;
mod wikifruit;

pub use self::debug::DebugProvider;
pub use self::fixed::FixedProvider;
pub use self::id_multi::{IdMulti, ProviderDetails as IdMultiProviderDetails};
pub use self::keyword_filter::KeywordFilterProvider;
pub use self::multi::Multi;
pub use self::timeout::TimeoutProvider;
pub use self::wikifruit::WikiFruit;
