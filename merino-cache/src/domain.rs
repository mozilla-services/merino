//! Data types specific to caching.

use merino_suggest::SuggestionRequest;
use std::borrow::Cow;

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
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.query.as_bytes());
        hasher.update(&[self.accepts_english as u8]);

        let hash = hasher.finalize().to_hex();
        format!("req:v2:{}", hash).into()
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
            accepts_english: true,
        };
        assert_eq!(
            req.cache_key(),
            "req:v2:9b32b37fe784364d9b5fa13d8a943b077ff853174887fc18c8d30fc2028fee5a",
        );
    }

    #[test]
    fn hash_uses_accepts_english_as_input() {
        let req1 = SuggestionRequest {
            query: "arbitrary".into(),
            accepts_english: true,
        };
        let req2 = SuggestionRequest {
            query: "arbitrary".into(),
            accepts_english: false,
        };

        assert_ne!(req1.cache_key(), req2.cache_key());
    }

    proptest! {
        /// Test that the cache key format is correct regardless of the input query.
        #[test]
        // "\\PC*" is a regex for any number of Printable Characters.
        fn key_format(query in "\\PC*", accepts_english in proptest::bool::ANY) {
            let req = SuggestionRequest {
                query: query.into(),
                accepts_english,
            };
            const HEX_DIGITS: &str = "0123456789abcdef";
            let parts: Vec<String> = req.cache_key().split(':').map(ToString::to_string).collect();
            prop_assert_eq!(parts.len(), 3);
            prop_assert_eq!(&parts[0], "req");
            prop_assert_eq!(&parts[1], "v2");
            prop_assert!(parts[2].chars().all(|c|HEX_DIGITS.contains(c)));
            prop_assert_eq!(parts[2].len(), 64);
        }
    }
}
