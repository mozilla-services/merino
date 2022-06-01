//! Testing utilities to work with logs.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    io::Write,
    ops::Deref,
    sync::{Arc, Mutex},
};
use tracing::Level;
use tracing_subscriber::fmt::MakeWriter;

/// Helper to collect events emitted by Tracing and later make assertions about
/// the collected events.
#[derive(Default)]
pub struct LogWatcher {
    /// The raw bytes received from Tracing. Should represent new-line separated JSON objects.
    buf: Arc<Mutex<Vec<u8>>>,

    /// Events serialized from [`buf`](Self::buf). As valid JSON objects are
    /// parsed from `buf`, the corresponding bytes are removed from `buf`. This
    /// way if there are any partial writes, only the complete objects are
    /// processed from the buffer, leaving incomplete objects in place.
    events: Vec<TracingJsonEvent>,
}

impl LogWatcher {
    /// Make a new LogWatcher with some events pre-populated. Primarily for testing LogWatcher itself.
    #[must_use]
    pub fn with_events(events: Vec<TracingJsonEvent>) -> Self {
        Self {
            events,
            buf: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Iterate over the events collected so far by this log watcher.
    pub fn events(&mut self) -> std::slice::Iter<TracingJsonEvent> {
        self.convert_events();
        self.events.iter()
    }

    /// Test if any event this logger received matches `predicate`.
    ///
    /// # Example
    ///
    /// ```
    /// # use merino_integration_tests::{LogWatcher, TracingJsonEvent};
    /// # use tracing::Level;
    /// # let mut fields = std::collections::HashMap::new();
    /// # fields.insert("message".to_string(), serde_json::json!("message".to_string()));
    /// # let mut log_watcher = LogWatcher::with_events(vec![
    /// #     TracingJsonEvent {
    /// #         fields,
    /// #         level: Level::INFO,
    /// #         target: String::new(),
    /// #         timestamp: String::new(),
    /// #     }
    /// # ]);
    /// #
    /// // assert!(log_watcher.has(|msg| msg.field_contains("message", "request success")));
    /// ```
    #[must_use = "LogWatcher::has does not make assertions alone, you probably want to wrap it in assert!()"]
    pub fn has<F>(&mut self, predicate: F) -> bool
    where
        F: FnMut(&TracingJsonEvent) -> bool,
    {
        self.events().any(predicate)
    }

    /// Iterate through `self.buf` to convert newline separated, completed JSON
    /// objects into [`TracingJsonEvent`] instances that are placed in
    /// `self.events`.
    fn convert_events(&mut self) {
        let mut buf = self.buf.lock().expect("mutex was poisoned");
        let mut log_text = String::from_utf8(buf.clone()).expect("bad utf8");

        // Repeatedly find the next newline char...
        while let Some(idx) = log_text.find('\n') {
            // Split the string at that point...
            let mut message_json = log_text.split_off(idx);
            // and keep the left side, and return the right side to the string
            std::mem::swap(&mut message_json, &mut log_text);
            // Remove the leading newline we left there.
            assert_eq!(log_text.chars().next(), Some('\n'));
            log_text.remove(0);

            // Skip blank lines
            if message_json.trim().is_empty() {
                continue;
            }

            // Now `message_join` contains the first line of logs, and `log_text` contains the rest.
            let message: TracingJsonEvent = serde_json::from_str(&message_json)
                .unwrap_or_else(|_| panic!("Bad JSON in log line: {}", &message_json));
            self.events.push(message);
        }

        // Now put the rest of the text back into the buffer.
        *buf = log_text.into_bytes();
        // and the mutex unlocks when it drops at the end of the function.
    }
}

impl MakeWriter for LogWatcher {
    type Writer = LogWatcherWriter;

    fn make_writer(&self) -> Self::Writer {
        LogWatcherWriter {
            buf: self.buf.clone(),
        }
    }
}

/// A helper that collects log events emitted from Tracing.
///
/// This is needed because Tracing consumes its subscribers. This type is a
/// "scout" that is split off from the main [`LogWatcher`] to give to Tracing,
/// and the data is written back to the parent type.
#[derive(Clone)]
pub struct LogWatcherWriter {
    /// The handle to the parent log watcher's buffer.
    buf: Arc<Mutex<Vec<u8>>>,
}

impl Write for LogWatcherWriter {
    fn write(&mut self, new_bytes: &[u8]) -> std::io::Result<usize> {
        let mut buf = self
            .buf
            .lock()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;
        buf.extend(new_bytes.iter());
        Ok(new_bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A deserialization of [`tracing_subscriber::fmt::format::Json`]'s output format.
#[derive(Debug, Deserialize, Serialize)]
pub struct TracingJsonEvent {
    /// The key-value fields logged on the event, usually including `message`.
    pub fields: HashMap<String, Value>,
    /// The level the event was emitted at.
    #[serde(with = "serde_with::rust::display_fromstr")]
    pub level: Level,
    /// The target of the event.
    pub target: String,
    /// The time the event was emitted.
    pub timestamp: String,
}

impl TracingJsonEvent {
    /// Test if the field named `field_name` is a string that contains `pat` as a
    /// substring.
    pub fn field_contains<'a, S>(&'a self, field_name: &'a str, pat: S) -> bool
    where
        S: Deref<Target = str>,
    {
        self.fields
            .get(field_name)
            .and_then(serde_json::Value::as_str)
            .map_or(false, |value| value.contains(&*pat))
    }
}
