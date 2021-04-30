#![warn(missing_docs, clippy::missing_docs_in_private_items)]
#![cfg(test)]

//! Tests for Merino that work by reading from the external API only.
//!
//! Since the URL endpoints Merino exposes to the world are its public API, and
//! other systems depend on them, the paths used in tests here are important
//! details, and used to keep compatibility.
//!
//! This is structured as a separate crate so that it produces a single test
//! binary instead of one test per file like would happen if this were
//! `merino/tests/...`. This improves compilation and test times.

use httpmock::MockServer;
use reqwest::{Client, RequestBuilder};
use std::{net::TcpListener, sync::Once};

use merino_settings::Settings;

mod debug;
mod dockerflow;
mod suggest;

static ONE_TIME_INIT: Once = Once::new();

#[non_exhaustive]
pub(crate) struct TestingTools {
    test_client: TestReqwestClient,
    remote_settings_mock: MockServer,
}

impl TestingTools {
    /// Start the fully configured application server.
    ///
    /// The server will listen on a port assigned arbitrarily by the OS. A test HTTP
    /// client that automatically targets the server will be returned.
    pub(crate) fn new<F: FnOnce(&mut Settings)>(settings_changer: F) -> Self {
        // Set up a mock server for remote settings to talk to
        let remote_settings_mock = MockServer::start();

        // remote_settings_client uses viaduct. Tell viaduct to use reqwest.
        ONE_TIME_INIT.call_once(|| {
            viaduct::set_backend(&viaduct_reqwest::ReqwestBackend)
                .expect("Failed to set viaduct backend");
        });

        let settings = Settings::load_for_tests(|settings| {
            settings.adm.remote_settings.server = Some(remote_settings_mock.url(""));
            settings_changer(settings);
        });
        let listener = TcpListener::bind(settings.http.listen).expect("Failed to bind to a port");
        let address = listener.local_addr().unwrap().to_string();
        let server = merino_web::run(listener, settings).expect("Failed to start server");

        // Run the server in the background
        tokio::spawn(server);

        let test_client = TestReqwestClient::new(address);

        Self {
            test_client,
            remote_settings_mock,
        }
    }
}

/// A wrapper around a `[reqwest::client]` that automatically sends requests to
/// the test server.
///
/// Note: This only handles `GET` requests right now. Other methods should be
/// added as needed.
pub(crate) struct TestReqwestClient {
    client: Client,
    address: String,
}

impl TestReqwestClient {
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
