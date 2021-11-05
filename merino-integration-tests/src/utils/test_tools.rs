//! Tools for running tests

use crate::utils::{logging::LogWatcher, metrics::MetricsWatcher, redis::get_temp_db};
use merino_settings::Settings;
use reqwest::{
    multipart::{Form, Part},
    redirect, Client, ClientBuilder, RequestBuilder,
};
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

    // Set up Remote Settings
    let reqwest_client = reqwest::Client::new();
    let bucket_info: serde_json::Value = reqwest_client
        .post(format!("{}/v1/buckets/", settings.remote_settings.server))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .expect("Error creating bucket")
        .json()
        .await
        .expect("getting new bucket info");
    let bucket_name = bucket_info["data"]["id"].as_str().unwrap();
    let collection_info: serde_json::Value = reqwest_client
        .post(format!(
            "{}/v1/buckets/{}/collections",
            settings.remote_settings.server, bucket_name
        ))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .expect("Error creating collection")
        .json()
        .await
        .expect("getting new collection info");
    let collection_name = collection_info["data"]["id"].as_str().unwrap();

    settings.remote_settings.default_bucket = bucket_name.to_string();
    settings.remote_settings.default_collection = collection_name.to_string();

    settings_changer(&mut settings);
    if let Some(changes) = &settings.remote_settings.test_changes {
        setup_remote_settings_collection(
            &settings.remote_settings,
            &changes.iter().map(String::as_str).collect::<Vec<_>>(),
        )
        .await;
    }

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
    let providers = merino_web::providers::SuggestionProviderRef::init(&settings, &metrics_client)
        .await
        .expect("Could not create providers");
    let server = merino_web::run(listener, metrics_client, settings.clone(), providers)
        .expect("Failed to start server");
    let server_handle = tokio::spawn(server.with_current_subscriber());
    let test_client = TestReqwestClient::new(address);

    // Assemble the tools
    let tools = TestingTools {
        test_client,
        settings,
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

    /// A copy of the current settings.
    pub settings: Settings,

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

/// Create the remote settings collection and endpoint from the provided suggestions
pub async fn setup_remote_settings_collection(
    rs_settings: &merino_settings::RemoteSettingsGlobalSettings,
    suggestions: &[&str],
) {
    let client = reqwest::Client::new();
    let collection_url = format!(
        "{}/v1/buckets/{}/collections/{}",
        rs_settings.server, rs_settings.default_bucket, rs_settings.default_collection
    );
    for (idx, s) in suggestions.iter().enumerate() {
        let attachment = serde_json::to_string(&json!([{
            "id": idx,
            "url": format!("https://example.com/#url/{}", s),
            "click_url": format!("https://example.com/#click/{}", s),
            "impression_url": format!("https://example.com/#impression/{}", s),
            "iab_category": "5 - Education",
            "icon": "1",
            "advertiser": "fake",
            "title": format!("Suggestion {}", s),
            "keywords": [s],
        }]))
        .expect("attachment json");

        let record = serde_json::to_string(&json!({
            "id": format!("suggestion-{}", s), "type": "data"
        }))
        .expect("record json");

        let url = format!("{}/records/suggestion-{}/attachment", collection_url, s);
        let req = client
            .post(&url)
            .multipart(
                Form::new()
                    .part(
                        "attachment",
                        Part::text(attachment).file_name(format!("suggestion-{}.json", idx)),
                    )
                    .part("data", Part::text(record)),
            )
            .send()
            .await
            .expect("making record");
        assert_eq!(req.status(), 201);
    }

    // make fake icon record
    let icon_record = serde_json::to_string(&json!({"id": "icon-1", "type": "icon"})).unwrap();

    let req = client
        .post(format!("{}/records/icon-1/attachment", collection_url))
        .multipart(
            Form::new()
                .part("attachment", Part::text("").file_name("icon.png"))
                .part("data", Part::text(icon_record)),
        )
        .send()
        .await
        .expect("create attachment");
    assert_eq!(req.status(), 201);
}
