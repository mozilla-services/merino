#![warn(missing_docs, clippy::missing_docs_in_private_items)]
// None of the tests are seen by the linter, so none of the utilities are marked
// as used. But docs don't generate for the below if they are `#[cfg(test)]`.
// This is a compromise.
#![allow(dead_code)]

//! Tests for Merino that work by reading from the external API only.
//!
//! Since the URL endpoints Merino exposes to the world are its public API, and
//! other systems depend on them, the paths used in tests here are important
//! details, and used to keep compatibility.
//!
//! This is structured as a separate crate so that it produces a single test
//! binary instead of one test per file like would happen if this were
//! `merino/tests/...`. This improves compilation and test times.
//!
//! The primary tool used by tests is [`merino_test`], which creates mock
//! servers, sets up the application for testing, and provides helpers to inspect
//! the state of the app. It then calls the test function that is passed to it,
//! providing the above tools as an argument.
//!
//! ```
//! use merino_integration_tests::{TestingTools, merino_test_macro};
//!
//! #[merino_test_macro(|settings| settings.debug = true)]
//! async fn lbheartbeat_works(TestingTools { test_client, .. }: TestingTools) {
//!    let response = test_client
//!        .get("/__lbheartbeat__")
//!        .send()
//!        .await
//!        .expect("failed to execute request");
//!
//!    assert_eq!(response.status(), StatusCode::OK);
//!    assert_eq!(response.content_length(), Some(0));
//! }
//! ```

mod caching;
mod debug;
mod dockerflow;
mod general;
mod logging;
mod suggest;
mod utils;

pub use crate::utils::{
    logging::{LogWatcher, TracingJsonEvent},
    metrics::MetricsWatcher,
    test_tools::{merino_test, TestingTools},
};

pub use merino_integration_tests_macro::merino_test as merino_test_macro;
