//! # Testing strategies
//!
//! There are two major testing strategies used in this repository: unit tests,
//! and integration tests.
//!
//! Unit tests should appear close to the code they are testing, using standard
//! Rust unit tests. This is suitable for testing complex behavior at a small
//! scale, with fine grained control over the inputs.
//!
//! ```
//! fn add_two(n: u32) -> u32 {
//!     n + 2
//! }
//!
//! #[cfg(test)]
//! mod tests {
//!     #[test]
//!     fn add_two_works() {
//!         assert_eq!(add_two(3), 5, "it should work");
//!     }
//! }
//! ```
//!
//! However, many behaviors are difficult to test as unit tests, especially
//! details like the URLs we expose via the web service. To test these parts of
//! Merino, we have [`merino-integration-tests`][test-crate], which starts an
//! instance of Merino with mock data sources. HTTP requests can then be made to
//! that server in order to test its behavior.
//!
//! [test-crate]: ../../../merino_integration_tests/
//!
//! ```ignore
//! #[actix_rt::test]
//! async fn lbheartbeat_works() {
//!     merino_test(
//!         |_| (),
//!         |TestingTools { test_client, .. }| async move {
//!             let response = test_client
//!                 .get("/__lbheartbeat__")
//!                 .send()
//!                 .await
//!                 .expect("failed to execute request");
//!
//!             assert_eq!(response.status(), StatusCode::OK);
//!             assert_eq!(response.content_length(), Some(0));
//!         },
//!     )
//!     .await
//! }
//! ```
//!
//! For more details, see the documentation of the `merino-integration-tests`
//! crate.
