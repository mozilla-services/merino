//! Additional tools for recording metrics from suggestion providers.

use cadence::{Histogram, Histogrammed, MetricBuilder, MetricResult};
use std::time::Duration;

/// Trait for recording timer values with additional precision.
///
/// Time data is stored as a histogram.  Statistical distribution is calculated
/// by the server. Times will be stored a number of milliseconds, with
/// fractional values used to represent nanoseconds.
///
/// Only `Duration types are valid.
///
/// See the [Statsd spec](https://github.com/b/statsd_spec) for more
/// information.
///
/// Note that tags and histograms are a
/// [Datadog](https://docs.datadoghq.com/developers/dogstatsd/) extension to
/// Statsd and may not be supported by your server.
pub trait TimedMicros {
    /// Record a single histogram value with the given key
    /// # Errors
    /// Returns an error if there was a problem sending the metric.
    fn time_micros(&self, key: &str, value: Duration) -> MetricResult<Histogram> {
        self.time_micros_with_tags(key, value).try_send()
    }

    /// Record a single histogram value with the given key and return a
    /// `MetricBuilder` that can be used to add tags to the metric.
    fn time_micros_with_tags<'a>(
        &'a self,
        key: &'a str,
        value: Duration,
    ) -> MetricBuilder<'_, '_, Histogram>;
}

impl<C: Histogrammed<f64>> TimedMicros for C {
    fn time_micros_with_tags<'a>(
        &'a self,
        key: &'a str,
        value: Duration,
    ) -> MetricBuilder<'_, '_, Histogram> {
        debug_assert!(key.ends_with("-us"));

        let nanos = value.as_nanos() as f64;
        let millis = nanos / 1000_f64;
        self.histogram_with_tags(key, millis)
    }
}
