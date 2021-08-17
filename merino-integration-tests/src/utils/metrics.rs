//! Tools to help testing metrics

use cadence::{SpyMetricSink, StatsdClient};
use crossbeam_channel::Receiver;
use statsd_parser::Message;

/// Helper to collect metrics during tests, and make assertions about them.
pub struct MetricsWatcher {
    /// Crossbeam channel that receives metrics lines as bytes.
    rx: Receiver<Vec<u8>>,

    /// Metrics received by the watcher from [`rx`].
    messages: Vec<statsd_parser::Message>,
}

impl MetricsWatcher {
    /// Make a new metrics watcher, attach it to a [`StatsdClient`] and return both.
    pub fn new_with_client() -> (Self, StatsdClient) {
        let (rx, spy_sink) = SpyMetricSink::new();
        let metrics_client = cadence::StatsdClient::from_sink("", spy_sink);
        let metrics_watcher = Self {
            rx,
            messages: vec![],
        };

        (metrics_watcher, metrics_client)
    }

    /// Consume any waiting events from `rx` and parse them as metrics.
    fn process_events(&mut self) {
        self.messages.extend(self.rx.try_iter().map(|bytes| {
            let s = String::from_utf8(bytes).expect("Invalid UTF8 in metric message");
            statsd_parser::parse(s).expect("Metric message parse error")
        }));
    }

    /// Get a list of all the metrics seen by this watcher, primarily for debugging.
    pub fn all_messages(&mut self) -> &[statsd_parser::Message] {
        self.process_events();
        self.messages.as_slice()
    }

    /// Test if any metric this watcher received matches `predicate`.
    ///
    /// # Example
    ///
    /// ```
    /// # use merino_integration_tests::MetricsWatcher;
    /// # use cadence::CountedExt;
    /// # let (mut metrics_watcher, metrics_client) = MetricsWatcher::new_with_client();
    /// #
    /// use statsd_parser::{Message, Metric, Counter};
    /// metrics_client.incr("a-metric");
    ///
    /// assert!(metrics_watcher.has(|msg| {
    ///     println!("@@@ msg: {:?}", msg);
    ///     msg.name == "a-metric"
    ///         && matches!(msg.metric, Metric::Counter(Counter { value, .. }) if value == 1.0)
    /// }));
    /// ```
    pub fn has<F>(&mut self, predicate: F) -> bool
    where
        F: FnMut(&Message) -> bool,
    {
        self.all_messages().iter().any(predicate)
    }

    /// Test if any metric this watcher received was a histogram with the given name and value.
    ///
    /// Values are compared by taking the absolute difference between them, and
    /// checking if it less than an epsilon of 0.0001.
    pub fn has_histogram(&mut self, name: &str, expected_value: f64) -> bool {
        self.has(|msg| {
            msg.name == name
                && match &msg.metric {
                    statsd_parser::Metric::Histogram(histogram) => {
                        (histogram.value - expected_value).abs() <= 0.0001
                    }
                    _ => false,
                }
        })
    }
}
