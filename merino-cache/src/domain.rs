//! Data types specific to caching.

use highway::{HighwayHash, HighwayHasher};
use merino_suggest::SuggestionRequest;
use std::{borrow::Cow, hash::Hash};

/// An object that can generate a cache key for itself.
pub trait CacheKey<'a> {
    /// Generate a cache key for this object. Two objects that have the same
    /// cache key should be functionally identical.
    ///
    /// Cache keys should make it clear that they are cache keys, and specify the
    /// type of object they refer to. They should also include a version
    /// indicator. For example: `cache:req:v1:d1bc8d3ba4afc7e1`. Excessively long
    /// key lengths should be avoided. 100 bytes is a good upper bound.
    fn cache_key(&self) -> Cow<'a, str>;
}

impl<'a> CacheKey<'a> for SuggestionRequest<'a> {
    fn cache_key(&self) -> Cow<'a, str> {
        // Notably, this uses a default key of all zeroes. This is not hash-DoS
        // resistant. Consider making this a setting in the future.
        let mut hasher = HighwayHasher::default();
        self.hash(&mut hasher);
        let hash = hasher.finalize256();

        // Print the hash as a padded hex number. `0>16x` reads as: use zeroes
        // to right align to a width of 16 characters, in hexadecimal.
        format!(
            "req:v2:{:0>16x}{:0>16x}{:0>16x}{:0>16x}",
            hash[0], hash[1], hash[2], hash[3]
        )
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::CacheKey;
    use merino_suggest::SuggestionRequest;
    use proptest::prelude::*;

    /// This test provides a fixed input, and expects a certain cache key to be
    /// produced. This alerts us to any time the cache algorithm changes. If this
    /// is an expected change, you should increment the version number in the
    /// cache key string.
    #[test]
    fn it_works() {
        let req = SuggestionRequest {
            query: "arbitrary".into(),
        };
        assert_eq!(
            req.cache_key(),
            "req:v2:0f7d741a1a8c92b576e5af170f35761fa7670e4fa5b0861ee5f760a056c7d62b"
        );
    }

    proptest! {
        /// Test that the cache key format is correct regardless of the input query.
        #[test]
        // "\\PC*" is a regex for any number of Printable Characters.
        fn key_format(s in "\\PC*") {
            let req = SuggestionRequest {
                query: s.into(),
            };
            static HEX_DIGITS: &str = "0123456789abcdef";
            let parts: Vec<String> = req.cache_key().split(':').map(ToString::to_string).collect();
            prop_assert_eq!(parts.len(), 3);
            prop_assert_eq!(&parts[0], "req");
            prop_assert_eq!(&parts[1], "v2");
            prop_assert!(parts[2].chars().all(|c|HEX_DIGITS.contains(c)));
            prop_assert_eq!(parts[2].len(), 64);
        }
    }
}
