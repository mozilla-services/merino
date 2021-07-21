//! Data types specific to caching.

use std::{
    borrow::Cow,
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
};

use merino_suggest::SuggestionRequest;

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
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        let hash = hasher.finish();
        // Print the hash as a padded hex number. `0>16x` -> use zeroes to right
        // align to a width of 16 characters, in hexadecimal.
        format!("req:v1:{:0>16x}", hash).into()
    }
}

#[cfg(test)]
mod tests {
    use super::CacheKey;
    use merino_suggest::SuggestionRequest;
    use proptest::prelude::*;

    #[test]
    fn it_works() {
        let req = SuggestionRequest {
            query: "arbitrary".into(),
        };
        assert_eq!(req.cache_key(), "req:v1:62932cfb1976ac51");
    }

    proptest! {
        /// Test that the cache key format is correct regardless of the input query.
        #[test]
        // "\\PC*" is a regex for any number of Printable Characters.
        fn key_format(s in "\\PC*") {
            let req = SuggestionRequest {
                query: s.into(),
            };
            let hex_digits = "0123456789abcdef";
            let parts: Vec<String> = req.cache_key().split(':').map(ToString::to_string).collect();
            prop_assert_eq!(parts.len(), 3);
            prop_assert_eq!(&parts[0], "req");
            prop_assert_eq!(&parts[1], "v1");
            prop_assert!(parts[2].chars().all(|c| hex_digits.contains(c)));
        }
    }
}
