//! Datatypes to better represent the domain of Merino.

use anyhow::ensure;
use rand::distributions::{Distribution, Standard};
use serde::{de, Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};
use std::fmt::Debug;

/// Represents a value from 0.0 to 1.0, inclusive. That is: a portion of
/// something that cannot be negative or exceed 100%.
///
/// Stored internally as a u32.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Proportion(u32);

impl Proportion {
    /// The lowest value for a portion, corresponding to 0%.
    pub fn zero() -> Self {
        Proportion(0)
    }

    /// The highest value for a portion, corresponding to 100%.
    pub fn one() -> Self {
        Proportion(u32::MAX)
    }

    /// Converts a float value into a Proportion. Panics if the value is not
    /// between zero and one.
    ///
    /// This is not implemented using [`std::config::From`] because you cannot
    /// implement both Try and TryFrom for the same pair of types, due to a
    /// blanket `impl TryFor<T> for U where U: Try<T>`.
    pub fn from<T>(v: T) -> Self
    where
        T: TryInto<Self>,
        <T as TryInto<Self>>::Error: Debug,
    {
        v.try_into().unwrap()
    }
}

/// Implement traits for a float type.
macro_rules! impl_for_float {
    ($type: ty) => {
        impl TryFrom<$type> for Proportion {
            type Error = anyhow::Error;

            fn try_from(v: $type) -> Result<Self, Self::Error> {
                ensure!(!v.is_infinite(), "v cannot be infinite");
                ensure!(v >= 0.0, "v must be positive");
                ensure!(v <= 1.0, "v cannot be greater than 1");

                Ok(Self((v * (u32::MAX as $type)) as u32))
            }
        }

        impl From<Proportion> for $type {
            fn from(portion: Proportion) -> $type {
                (portion.0 as $type) / (u32::MAX as $type)
            }
        }

        impl From<&Proportion> for $type {
            fn from(portion: &Proportion) -> $type {
                (portion.0 as $type) / (u32::MAX as $type)
            }
        }
    };
}

impl_for_float!(f32);
impl_for_float!(f64);

impl Distribution<Proportion> for Standard {
    fn sample<R: rand::Rng + ?Sized>(&self, rng: &mut R) -> Proportion {
        Proportion(rng.gen())
    }
}

impl Serialize for Proportion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_f64(self.into())
    }
}

impl<'de> Deserialize<'de> for Proportion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        /// Visitor for deserializing a Proportion
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = Proportion;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "value between 0.0 and 1.0")
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v >= 0 {
                    self.visit_u64(v as u64)
                } else {
                    Err(de::Error::invalid_value(de::Unexpected::Signed(v), &self))
                }
            }

            // u8, u16, and u32 delegate to this
            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v == 0 {
                    Ok(Proportion::zero())
                } else if v == 1 {
                    Ok(Proportion::one())
                } else {
                    Err(de::Error::invalid_value(de::Unexpected::Unsigned(v), &self))
                }
            }

            // f32 delegates to this
            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                v.try_into()
                    .map_err(|_err| de::Error::invalid_value(de::Unexpected::Float(v), &self))
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

/// Gathers inputs to be hashed to determine a cache key.
pub trait CacheInputs {
    /// Add data to the cache key.
    fn add(&mut self, input: &[u8]);
    /// Generate a cache key from the collected inputs so far.
    fn hash(&self) -> String;
}

impl CacheInputs for blake3::Hasher {
    fn add(&mut self, input: &[u8]) {
        self.update(input);
    }

    fn hash(&self) -> String {
        self.finalize().to_hex().to_string()
    }
}
