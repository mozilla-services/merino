use redis::ConnectionInfo;
use serde::{Deserializer, Serializer};
use serde_with::{DeserializeAs, SerializeAs};

pub struct AsConnectionInfo;

impl SerializeAs<ConnectionInfo> for AsConnectionInfo {
    fn serialize_as<S>(value: &ConnectionInfo, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serde_redis_connection::serialize(value, serializer)
    }
}

impl<'de> DeserializeAs<'de, ConnectionInfo> for AsConnectionInfo {
    fn deserialize_as<D>(deserializer: D) -> Result<ConnectionInfo, D::Error>
    where
        D: Deserializer<'de>,
    {
        serde_redis_connection::deserialize(deserializer)
    }
}

pub mod serde_redis_connection {
    use redis::{ConnectionInfo, IntoConnectionInfo};
    use serde::{
        de::{Unexpected, Visitor},
        Deserializer, Serializer,
    };

    pub fn serialize<S>(connection_info: &ConnectionInfo, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(
            match connection_info {
                ConnectionInfo {
                    addr,
                    db,
                    username: Some(username),
                    passwd: Some(passwd),
                } => format!("redis://{}:{}@{}/{}", username, passwd, addr, db),
                ConnectionInfo {
                    addr,
                    db,
                    username: Some(username),
                    passwd: None,
                } => format!("redis://{}@{}/{}", username, addr, db),
                ConnectionInfo {
                    addr,
                    db,
                    username: None,
                    passwd: Some(passwd),
                } => format!("redis://:{}@{}/{}", passwd, addr, db),
                ConnectionInfo {
                    addr,
                    db,
                    username: None,
                    passwd: None,
                } => format!("redis://{}/{}", addr, db),
            }
            .as_str(),
        )
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<redis::ConnectionInfo, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct V;

        impl<'de> Visitor<'de> for V {
            type Value = ConnectionInfo;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a valid Redis connection info")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                v.into_connection_info()
                    .map_err(|_err| E::invalid_value(Unexpected::Str(v), &self))
            }
        }

        deserializer.deserialize_string(V)
    }
}
