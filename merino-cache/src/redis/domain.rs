//! Wrapper types for [`merino-suggest`] types so traits can be implemented

use merino_suggest::Suggestion;
use redis::{FromRedisValue, ToRedisArgs};

/// A byte string prepended to all cached data to avoid format problems. Cache
/// entries with a different cache key will be treated as invalid.
const SERIALIZATION_VERSION: &[u8] = b"v0";

/// A wrapper around a `Vec` of [`Suggestion`]s that can be inserted into and retrieved
/// from a Redis DB.
pub(crate) struct RedisSuggestions(pub(crate) Vec<Suggestion>);

impl FromRedisValue for RedisSuggestions {
    fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
        match v {
            redis::Value::Data(bytes) => {
                if &bytes[..2] != SERIALIZATION_VERSION {
                    return Err((
                        redis::ErrorKind::TypeError,
                        "Unexpected cache serialization version `{}`.",
                        String::from_utf8_lossy(&bytes[..2]).to_string(),
                    )
                        .into());
                }

                serde_json::from_slice(&bytes[2..])
                    .map_err(|error| {
                        (
                            redis::ErrorKind::TypeError,
                            "Could not deserialize data from Redis",
                            format!("{:?}", error),
                        )
                            .into()
                    })
                    .map(Self)
            }

            v => Err((
                redis::ErrorKind::TypeError,
                "Invalid type received from Redis. Expected `Data`",
                format!("Got {:?}", v),
            )
                .into()),
        }
    }
}

impl ToRedisArgs for RedisSuggestions {
    fn write_redis_args<W>(&self, out: &mut W)
    where
        W: ?Sized + redis::RedisWrite,
    {
        let serialized = serde_json::to_string(&self.0).expect("Bug: cannot serialize suggestions");
        let mut output = String::from_utf8(SERIALIZATION_VERSION.to_vec())
            .expect("Bug: serialization version is not utf8");
        output.push_str(&serialized);
        out.write_arg(output.as_bytes())
    }
}

impl From<RedisSuggestions> for Vec<Suggestion> {
    fn from(val: RedisSuggestions) -> Self {
        val.0
    }
}

/// The result from the Redis `TTL` command, converting the two error codes (-1 and -2) into enum variants.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RedisTtl {
    /// The key requested does not exist.
    KeyDoesNotExist,
    /// The key requested exists, but does not have a TTL.
    KeyHasNoTtl,
    /// The TTL of the requested key.
    Ttl(usize),
}

impl FromRedisValue for RedisTtl {
    fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
        match v {
            redis::Value::Int(ttl) if *ttl >= 0 => Ok(Self::Ttl(*ttl as usize)),
            redis::Value::Int(ttl) if *ttl == -1 => Ok(Self::KeyHasNoTtl),
            redis::Value::Int(ttl) if *ttl == -2 => Ok(Self::KeyDoesNotExist),

            redis::Value::Int(_) => Err((
                redis::ErrorKind::TypeError,
                "Invalid value received from Redis. Expected a non-negative integer, -1, or -2",
            )
                .into()),

            _ => Err((
                redis::ErrorKind::TypeError,
                "Invalid type received from Redis. Expected `Int`",
            )
                .into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use anyhow::{anyhow, Result};
    use http::Uri;
    use merino_suggest::Suggestion;
    use proptest::prelude::*;
    use redis::{FromRedisValue, ToRedisArgs};

    use crate::redis::domain::{RedisSuggestions, RedisTtl, SERIALIZATION_VERSION};

    #[test]
    fn can_convert_redis_value_to_suggestions() -> Result<()> {
        let original_suggestions = vec![Suggestion {
            id: 1,
            full_keyword: "one".to_string(),
            title: "One".to_string(),
            url: Uri::from_static("https://example.com/target/one"),
            impression_url: Uri::from_static("https://example.com/impression/one"),
            click_url: Uri::from_static("https://example.com/click/one"),
            advertiser: "One Inc.".to_string(),
            is_sponsored: true,
            icon: Uri::from_static("https://example.com/icon/one.png"),
        }];

        let mut serialized = SERIALIZATION_VERSION.to_vec();
        serialized.extend_from_slice(&serde_json::to_vec(&original_suggestions)?);
        let redis_value = redis::Value::Data(serialized);
        let deserialized = RedisSuggestions::from_redis_value(&redis_value)?;

        assert_eq!(original_suggestions, deserialized.0);
        Ok(())
    }

    #[test]
    fn invalid_serialization_version_produces_an_error() -> Result<()> {
        let redis_value = redis::Value::Data("vXbad version".as_bytes().to_vec());
        let error = RedisSuggestions::from_redis_value(&redis_value)
            .err()
            .ok_or_else(|| anyhow!("expected error"))?;
        assert_eq!(error.kind(), redis::ErrorKind::TypeError);
        assert!(error.to_string().contains("version"));
        assert_eq!(
            error.detail().ok_or_else(|| anyhow!("no error detail"))?,
            "vX"
        );
        Ok(())
    }

    #[test]
    fn invalid_serialization_json_produces_an_error() -> Result<()> {
        let redis_value = redis::Value::Data(r#"v0["bad_json"#.as_bytes().to_vec());
        let error = RedisSuggestions::from_redis_value(&redis_value)
            .err()
            .ok_or_else(|| anyhow!("expected error"))?;
        dbg!(&error);
        assert_eq!(error.kind(), redis::ErrorKind::TypeError);
        assert!(error.to_string().contains("Could not deserialize"));
        assert!(error
            .detail()
            .ok_or_else(|| anyhow!("no error detail"))?
            .contains("EOF while parsing a string"));
        Ok(())
    }

    #[test]
    fn other_redis_values_cannot_be_converted_to_suggestions() -> Result<()> {
        assert!(RedisSuggestions::from_redis_value(&redis::Value::Nil).is_err());
        assert!(RedisSuggestions::from_redis_value(&redis::Value::Int(42)).is_err());
        assert!(RedisSuggestions::from_redis_value(&redis::Value::Bulk(vec![])).is_err());
        assert!(RedisSuggestions::from_redis_value(&redis::Value::Status(
            "unexpected".to_string()
        ))
        .is_err());
        assert!(RedisSuggestions::from_redis_value(&redis::Value::Okay).is_err());
        Ok(())
    }

    #[test]
    fn can_convert_suggestions_to_redis_arg() -> Result<()> {
        let original_suggestions = vec![Suggestion {
            id: 1,
            full_keyword: "one".to_string(),
            title: "One".to_string(),
            url: Uri::from_static("https://example.com/target/one"),
            impression_url: Uri::from_static("https://example.com/impression/one"),
            click_url: Uri::from_static("https://example.com/click/one"),
            advertiser: "One Inc.".to_string(),
            is_sponsored: true,
            icon: Uri::from_static("https://example.com/icon/one.png"),
        }];

        let val = RedisSuggestions(original_suggestions.clone()).to_redis_args();
        assert_eq!(val.len(), 1);
        assert_eq!(&val[0][..2], SERIALIZATION_VERSION);
        let parsed_suggestions: Vec<Suggestion> = serde_json::from_slice(&val[0][2..])?;

        assert_eq!(parsed_suggestions, original_suggestions);
        Ok(())
    }

    #[test]
    fn from_redis_suggestion_for_vec_suggestions() {
        let suggestions = RedisSuggestions(vec![]);
        let expected: Vec<Suggestion> = vec![];
        let actual: Vec<Suggestion> = suggestions.into();
        assert_eq!(actual, expected);
    }

    #[test]
    fn can_convert_redis_value_to_ttl() -> Result<()> {
        assert_eq!(
            RedisTtl::from_redis_value(&redis::Value::Int(-2))?,
            RedisTtl::KeyDoesNotExist
        );
        assert_eq!(
            RedisTtl::from_redis_value(&redis::Value::Int(-1))?,
            RedisTtl::KeyHasNoTtl
        );
        assert_eq!(
            RedisTtl::from_redis_value(&redis::Value::Int(0))?,
            RedisTtl::Ttl(0)
        );
        Ok(())
    }

    #[test]
    fn ttl_type_errors_are_handled() {
        assert!(RedisTtl::from_redis_value(&redis::Value::Nil).is_err());
        assert!(RedisTtl::from_redis_value(&redis::Value::Okay).is_err());
        assert!(
            RedisTtl::from_redis_value(&redis::Value::Status("unexpected".to_string())).is_err()
        );
        assert!(RedisTtl::from_redis_value(&redis::Value::Data(b"unexpected".to_vec())).is_err());
        assert!(RedisTtl::from_redis_value(&redis::Value::Bulk(vec![])).is_err());
    }

    proptest! {
        /// Test that valid TTLs are handled
        #[test]
        fn can_convert_valid_ttl_values(ttl in 0_i64..) {
            let res = RedisTtl::from_redis_value(&redis::Value::Int(ttl));
            prop_assert_eq!(res, Ok(RedisTtl::Ttl(ttl as usize)));
        }

        /// Test that invalid TTLs produce errors
        #[test]
        fn invalid_ttls_make_errors(ttl in ..-2_i64) {
            let res = RedisTtl::from_redis_value(&redis::Value::Int(ttl));
            prop_assert_eq!(res.err().unwrap().kind(), redis::ErrorKind::TypeError);

        }
    }
}
