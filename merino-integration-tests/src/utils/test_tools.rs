//! Tools for running tests

use crate::utils::{logging::LogWatcher, metrics::MetricsWatcher, redis::get_temp_db};
use httpmock::MockServer;
use merino_settings::Settings;
use reqwest::{redirect, Client, ClientBuilder, RequestBuilder};
use serde_json::json;
use std::{future::Future, net::TcpListener};
use tracing_futures::{Instrument, WithSubscriber};

use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt};

/// Run a test with a fully configured Merino server.
///
/// The server will listen on a port assigned arbitrarily by the OS.
///
/// A suite of tools will be passed to the test function in the form of an
/// instance of [`TestingTools`]. It includes an HTTP client configured to use
/// the test server, an HTTP mock server that Remote Settings has been configured
/// to read from, and a log collector that can make assertions about logs that
/// were printed.
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
///
/// # Panics
/// May panic if tests could not be set up correctly.
pub async fn merino_test<FSettings, FTest, Fut>(
    settings_changer: FSettings,
    test: FTest,
) -> Fut::Output
where
    FSettings: FnOnce(&mut Settings),
    FTest: Fn(TestingTools) -> Fut,
    Fut: Future,
{
    let test_span = tracing::info_span!("merino_test", redis_db = tracing::field::Empty);

    // Load settings
    let mut settings = Settings::load_for_tests();

    // Set up logging
    let log_watcher = LogWatcher::default();
    let log_watcher_writer = log_watcher.make_writer();

    let env_filter: tracing_subscriber::EnvFilter = (&settings.logging.levels).into();
    let tracing_subscriber = tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(move || log_watcher_writer.clone()),
        )
        .with(tracing_subscriber::fmt::layer().pretty().with_test_writer());

    let _tracing_subscriber_guard = tracing::subscriber::set_default(tracing_subscriber);

    // Set up a mock server for Remote Settings to talk to
    let remote_settings_mock = MockServer::start();
    remote_settings_mock.mock(|when, then| {
        when.path("/v1/");
        then.status(200)
            .header("Content-Type", "application/json")
            .json_body(json!({
                "capabilities": {
                    "attachments": {
                        "base_url": remote_settings_mock.base_url()
                    }
                }
            }));
    });
    settings.remote_settings.server = remote_settings_mock.base_url();

    // Set up Redis
    let _redis_connection_guard = match get_temp_db(&settings.redis.url).await {
        Ok((connection_info, connection_guard)) => {
            tracing::debug!(?connection_info, "Configuring temporary Redis database");
            settings.redis.url = connection_info;
            Some(connection_guard)
        }
        Err(error) => {
            tracing::warn!(%error, "Could not set up Redis for test");
            None
        }
    };

    settings_changer(&mut settings);

    // Setup metrics
    assert_eq!(
        settings.metrics.sink_host, "0.0.0.0",
        "Tests cannot change the metrics sink host, since it is ignored"
    );
    assert_eq!(
        settings.metrics.sink_port, 8125,
        "Tests cannot change the metrics sink address, since it is ignored"
    );
    let (metrics_watcher, metrics_client) = MetricsWatcher::new_with_client();

    // Run server in the background
    let listener = TcpListener::bind(settings.http.listen).expect("Failed to bind to a port");
    let address = listener.local_addr().unwrap().to_string();
    let redis_client =
        redis::Client::open(settings.redis.url.clone()).expect("Couldn't access redis server");
    let server =
        merino_web::run(listener, metrics_client, settings).expect("Failed to start server");
    let server_handle = tokio::spawn(server.with_current_subscriber());
    let test_client = TestReqwestClient::new(address);

    // Assemble the tools
    let tools = TestingTools {
        test_client,
        remote_settings_mock,
        log_watcher,
        redis_client,
        metrics_watcher,
    };
    // Run the test
    let rv = test(tools).instrument(test_span).await;
    server_handle.abort();
    rv
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
    /// be added
    ///
    /// The server will listen on a port assigned arbitrarily by the OS. A test HTTP
    /// client that automatically targets the server will be returned.
    pub remote_settings_mock: MockServer,

    /// To make assertions about logs.
    pub log_watcher: LogWatcher,

    /// To interact with the Redis cache
    pub redis_client: redis::Client,

    /// To make assertions about metrics.
    pub metrics_watcher: MetricsWatcher,
}

/// A wrapper around a `[reqwest::client]` that automatically sends requests to
/// the test server.
///
/// This only handles `GET` requests right now. Other methods should be
/// added as needed.
///
/// The client is configured to not follow any redirects.
pub struct TestReqwestClient {
    /// The wrapped client.
    client: Client,

    /// The server address to implicitly use for all requests.
    address: String,
}

impl TestReqwestClient {
    /// Construct a new test client that uses `address` for every request given.
    pub fn new(address: String) -> Self {
        let client = ClientBuilder::new()
            .redirect(redirect::Policy::none())
            .build()
            .expect("Could not build test client");
        Self { client, address }
    }

    /// Start building a GET request to the test server with the path specified.
    ///
    /// The path should start with `/`, such as `/__heartbeat__`.
    pub fn get(&self, path: &str) -> RequestBuilder {
        assert!(path.starts_with('/'));
        let url = format!("http://{}{}", &self.address, path);
        self.client.get(url)
    }
}
