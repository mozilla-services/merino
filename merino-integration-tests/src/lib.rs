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

mod debug;
mod dockerflow;
mod logging;
mod suggest;
pub mod utils;

use crate::utils::logging::LogWatcher;
use httpmock::MockServer;
use merino_settings::Settings;
use reqwest::{Client, RequestBuilder};
use std::{net::TcpListener, sync::Once};

use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt};

/// A marker to track that the viaduct backend has been initialized.
static VIADUCT_INIT: Once = Once::new();

/// Run a test with a full configured Merino server.
///
/// The server will listen on a port assigned arbitrarily by the OS.
///
/// A suite of tools will be passed to the test function, which will include an
/// HTTP client configured to use the test server, an HTTP mock server that
/// Remote Settings has been configured to read from, and a log collector that
/// can make assertions about logs that were printed.
///
/// # Example
///
/// ```
/// # use merino_integration_tests::{merino_test, TestingTools};
/// #[actix_rt::test]
/// async fn a_test() {
///     merino_test(
///         |settings| settings.debug = false,
///         |TestingTools { test_client, mut log_watcher, .. }| async move {
///             assert!(true) // Test goes here
///         }
///     ).await
/// }
/// ```
pub fn merino_test<FSettings, FTest, R>(settings_changer: FSettings, test: FTest) -> R
where
    FSettings: FnOnce(&mut Settings),
    FTest: Fn(TestingTools) -> R,
{
    // Set up a mock server for Remote Settings to talk to
    let remote_settings_mock = MockServer::start();

    // Load settings
    let settings = Settings::load_for_tests(|settings| {
        settings.adm.remote_settings.server = Some(remote_settings_mock.url(""));
        settings_changer(settings);
    });

    // `remote_settings_client` uses viaduct. Tell viaduct to use reqwest.
    VIADUCT_INIT.call_once(|| {
        viaduct::set_backend(&viaduct_reqwest::ReqwestBackend)
            .expect("Failed to set viaduct backend");
    });

    // Set up logging
    let log_watcher = LogWatcher::default();
    let log_watcher_writer = log_watcher.make_writer();

    let subscriber = tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(move || log_watcher_writer.clone()),
        )
        .with(tracing_subscriber::fmt::layer().pretty().with_test_writer());

    // Run server in the background
    let listener = TcpListener::bind(settings.http.listen).expect("Failed to bind to a port");
    let address = listener.local_addr().unwrap().to_string();
    let server = merino_web::run(listener, settings).expect("Failed to start server");
    tokio::spawn(server);
    let test_client = TestReqwestClient::new(address);

    // Assemble the tools
    let tools = TestingTools {
        test_client,
        remote_settings_mock,
        log_watcher,
    };

    // Run the test
    tracing::subscriber::with_default(subscriber, || test(tools))
}

/// A set of tools for tests, including mock servers and logging helpers.
///
/// The fields of this struct are marked as non-exhaustive, meaning that any
/// destructuring of this struct will require a `..` "and the rest" entry, even
/// if all present items are named. This makes adding tools in the future easier,
/// since old tests won't need to be rewritten to account for the added tools.
#[non_exhaustive]
pub struct TestingTools {
    /// A wrapper around a `reqwest::client` that automatically uses the Merino
    /// server under test.
    pub test_client: TestReqwestClient,

    /// A [`httpmock::MockServer`] that remote settings has been configured to use
    /// as its default server. Does not contain mock responses, any needed must
    /// be adde    /// Start the fully configured application server.
    ///
    /// The server will listen on a port assigned arbitrarily by the OS. A test HTTP
    /// client that automatically targets the server will be returned.
    pub remote_settings_mock: MockServer,

    /// To make assertions about logs.
    pub log_watcher: LogWatcher,
}

/// A wrapper around a `[reqwest::client]` that automatically sends requests to
/// the test server.
///
/// Note: This only handles `GET` requests right now. Other methods should be
/// added as needed.
pub struct TestReqwestClient {
    /// The wrapped client.
    client: Client,

    /// The server address to implicitly use for all requests.
    address: String,
}

impl TestReqwestClient {
    /// Construct a new test client that uses `address` for every request given.
    fn new(address: String) -> Self {
        Self {
            address,
            client: Client::new(),
        }
    }

    /// Start building a GET request to the test server with the path specified.
    ///
    /// The path should start with `/`, such as `/__heartbeat__`.
    fn get(&self, path: &str) -> RequestBuilder {
        assert!(path.starts_with('/'));
        let url = format!("http://{}{}", &self.address, path);
        self.client.get(url)
    }
}
