//! Data types specific to caching.

use merino_suggest::SuggestionRequest;

/// An object that can generate a cache key for itself.
pub trait CacheKey {
    /// Generate a cache key for this object. Two objects that have the same
    /// cache key should be functionally identical.
    ///
    /// Cache keys should make it clear that they are cache keys, and specify the
    /// type of object they refer to. They should also include a version
    /// indicator. For example: `cache:req:v1:d1bc8d3ba4afc7e1`. Excessively long
    /// key lengths should be avoided. 100 bytes is a good upper bound.
    fn cache_key(&self) -> String;
}

impl CacheKey for SuggestionRequest {
    fn cache_key(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.query.as_bytes());
        hasher.update(&[self.accepts_english as u8]);
        hasher.update(self.device_info.to_string().as_bytes());

        let hash = hasher.finalize().to_hex();
        format!("req:v3:{}", hash)
    }
}

#[cfg(test)]
mod tests {
    use super::CacheKey;
    use fake::{Fake, Faker};
    use merino_suggest::{
        device_info::{Browser, DeviceInfo, FormFactor, OsFamily},
        SuggestionRequest, FIREFOX_TEST_VERSIONS,
    };
    use proptest::prelude::*;
    use uuid::Uuid;

    /// This test provides a fixed input, and expects a certain cache key to be
    /// produced. This alerts us to any time the cache algorithm changes. If this
    /// is an expected change, you should increment the version number in the
    /// cache key string.
    #[test]
    fn it_works() {
        let req = SuggestionRequest {
            query: "arbitrary".into(),
            accepts_english: true,
            country: Some("US".into()),
            region: Some("OR".into()),
            dma: Some(820_u16),
            city: Some("Portland".into()),
            device_info: DeviceInfo {
                os_family: OsFamily::Windows,
                form_factor: FormFactor::Desktop,
                browser: Browser::Firefox(90),
            },
            request_id: Uuid::new_v4(),
        };
        assert_eq!(
            req.cache_key(),
            "req:v3:3096f07f8bce1cd4d39f2ea5544cd58e5c2ec94d3b491004a83257bb48c5fa45",
        );
    }

    #[test]
    fn hash_uses_accepts_english_as_input() {
        let req1 = SuggestionRequest {
            accepts_english: true,
            ..Faker.fake()
        };
        let req2 = SuggestionRequest {
            accepts_english: false,
            ..Faker.fake()
        };

        assert_ne!(req1.cache_key(), req2.cache_key());
    }

    proptest! {
        /// Test that the cache key format is correct regardless of the input query.
        #[test]
        // "\\PC*" is a regex for any number of Printable Characters.
        fn key_format(
            query in "\\PC*",
            accepts_english in proptest::bool::ANY,
            country in proptest::option::of("[A-Z]{2}"),
            region in proptest::option::of("[A-Z]{1,3}"),
            dma in proptest::option::of(100_u16..1000),
            city in proptest::option::of("[A-Z]{2}"),
            device_info in device_info_strategy()
        ) {
            let req = SuggestionRequest {
                query,
                accepts_english,
                country,
                region,
                dma,
                city,
                device_info,
                request_id: Uuid::new_v4(),
            };
            const HEX_DIGITS: &str = "0123456789abcdef";
            let parts: Vec<String> = req.cache_key().split(':').map(ToString::to_string).collect();
            prop_assert_eq!(parts.len(), 3);
            prop_assert_eq!(&parts[0], "req");
            prop_assert_eq!(&parts[1], "v3");
            prop_assert!(parts[2].chars().all(|c|HEX_DIGITS.contains(c)));
            prop_assert_eq!(parts[2].len(), 64);
        }
    }

    fn form_factor_strategy() -> impl Strategy<Value = FormFactor> {
        prop_oneof![
            Just(FormFactor::Desktop),
            Just(FormFactor::Phone),
            Just(FormFactor::Tablet),
            Just(FormFactor::Other),
        ]
    }

    fn os_family_strategy() -> impl Strategy<Value = OsFamily> {
        prop_oneof![
            Just(OsFamily::Windows),
            Just(OsFamily::MacOs),
            Just(OsFamily::Linux),
            Just(OsFamily::IOs),
            Just(OsFamily::Android),
            Just(OsFamily::ChromeOs),
            Just(OsFamily::BlackBerry),
            Just(OsFamily::Other),
        ]
    }

    fn browser_strategy() -> impl Strategy<Value = Browser> {
        prop_oneof![
            FIREFOX_TEST_VERSIONS.prop_map(Browser::Firefox),
            Just(Browser::Other),
        ]
    }

    prop_compose! {
        fn device_info_strategy()(
            form_factor in form_factor_strategy(),
            os_family in os_family_strategy(),
            browser in browser_strategy()
        ) -> DeviceInfo {
            DeviceInfo { form_factor, os_family, browser }
        }
    }
}
