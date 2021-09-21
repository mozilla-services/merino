use anyhow::{bail, Context};
use serde::{de, ser::SerializeSeq, Deserialize, Serialize};
use std::{ops::AddAssign, str::FromStr};
use tracing_subscriber::{filter::Directive, EnvFilter};

/// Logging settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSettings {
    /// The minimum level that logs should be reported at.
    ///
    /// Each entry can be one of `ERROR`, `WARN`, `INFO`, `DEBUG`, or `TRACE` (in
    /// increasing verbosity), with an optional component that specifies the
    /// source of the logs.
    ///
    /// This setting combined with the contents of the environment variable
    /// `RUST_LOG`, with values from the environment variable overriding the
    /// config file.
    ///
    /// The environment variable `MERINO_LOGGING__LEVELS` can be used. This
    /// environment variable will completely override the config file, and wil be
    /// merged with the envvar `RUST_LOG`. `RUST_LOG` takes precedence again.
    ///
    /// # Examples
    ///
    /// The configurations below are identical
    ///
    /// ```yaml
    /// # config/local.yaml
    /// logging:
    ///   levels:
    ///     - INFO              # default to INFO
    ///     - merino_web=DEBUG  # noisier logs from merino_web
    ///     - viaduct=WARN      # viaduct's INFO level is too noisy
    /// ```
    ///
    /// ```shell
    /// RUST_LOG=INFO,merino_web=DEBUG,viaduct=WARN
    /// ```
    pub levels: DirectiveWrapper,

    /// The format to output logs in.
    pub format: LogFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    /// [`tracing-subscriber`]'s human targeted, pretty format. Includes more
    /// information, Multiple lines per log event.
    Pretty,

    /// [MozLog](https://wiki.mozilla.org/Firefox/Services/Logging) JSON format.
    /// One line per log event.
    MozLog,

    /// [`tracing-subscriber`]'s default format. One line per log event.
    Compact,
}

/// Tracing's Directive object for filter logs isn't `Clone` or `Serializable`.
/// Make a wrapper for a collection of Directives so that we can work more easily
/// with it.
///
///This struct can be deserialized from either a comma separated string of
///directives (`"INFO,component1=WARN"`), or from a sequence of comma separated
///strings (`["INFO", "component1=WARN,component2=DEBUG"]`). This is important
///because the config files use sequences, but environment variables are always
///strings.
///
/// Every entry in this struct is guaranteed to be parsable as a valid Directive.
#[derive(Debug, Clone, PartialEq)]
pub struct DirectiveWrapper(Vec<String>);

impl Serialize for DirectiveWrapper {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.0.len()))?;
        for directive in &self.0 {
            seq.serialize_element(&directive)?;
        }
        seq.end()
    }
}

impl<'de> Deserialize<'de> for DirectiveWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = DirectiveWrapper;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "directive or list of directives")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                s.parse().map_err(|_err| {
                    de::Error::invalid_value(de::Unexpected::Str(s), &"valid directive")
                })
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut rv = DirectiveWrapper(vec![]);

                while let Some(item) = seq.next_element::<String>()? {
                    let parsed: DirectiveWrapper = item.parse().map_err(|err: anyhow::Error| {
                        de::Error::invalid_value(
                            de::Unexpected::Str(&item),
                            &err.to_string().as_str(),
                        )
                    })?;
                    rv += parsed;
                }

                Ok(rv)
            }
        }

        let mut rv = deserializer.deserialize_any(Visitor)?;

        // Add settings from RUST_LOG env var, which should always be respected
        if let Some(rust_log) = std::option_env!("RUST_LOG") {
            let from_env: DirectiveWrapper = rust_log.parse().map_err(|_err| {
                de::Error::invalid_value(de::Unexpected::Str(rust_log), &"valid directive")
            })?;
            rv += from_env;
        }

        Ok(rv)
    }
}

impl FromStr for DirectiveWrapper {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<String> = s.split(',').map(|s| s.to_string()).collect();

        // Test that each part can be parsed as a logging filter directive.
        if let Some(err) = parts.iter().find_map(|p| p.parse::<Directive>().err()) {
            return Err(err).context("valid syntax");
        }

        // directives with hyphens in them are foot-guns for us
        if parts.iter().any(|p| p.contains('-')) {
            bail!("log targets must not include hyphens");
        }

        // Wrap the string and return it.
        Ok(Self(parts))
    }
}

impl AddAssign for DirectiveWrapper {
    fn add_assign(&mut self, rhs: Self) {
        self.0.extend(rhs.0)
    }
}

impl From<&DirectiveWrapper> for EnvFilter {
    fn from(val: &DirectiveWrapper) -> Self {
        let mut rv = EnvFilter::default();
        for directive in &val.0 {
            rv = rv.add_directive(directive.parse().unwrap());
        }
        rv
    }
}
