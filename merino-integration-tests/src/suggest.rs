//! Tests Merino's ability to make basic suggestions.
#![cfg(test)]

use crate::{merino_test, TestingTools};
use anyhow::Result;
use httpmock::{Method::GET, MockServer};
use reqwest::StatusCode;
use serde_json::json;

#[actix_rt::test]
async fn suggest_wikifruit_works() -> Result<()> {
    merino_test(
        |_| (),
        |TestingTools {
             test_client,
             remote_settings_mock,
             ..
         }| async move {
            setup_empty_remote_settings_collection(remote_settings_mock);

            let response = test_client.get("/api/v1/suggest?q=apple").send().await?;

            assert_eq!(response.status(), StatusCode::OK);
            let body: serde_json::Value = response.json().await?;
            assert_eq!(
                body["suggestions"][0]["url"],
                json!("https://en.wikipedia.org/wiki/Apple")
            );

            Ok(())
        },
    )
    .await
}

fn setup_empty_remote_settings_collection(server: MockServer) {
    server.mock(|when, then| {
        when.method(GET)
            .path("/buckets/monitor/collections/changes/changeset");
        then.status(200).json_body(json!({
            "metadata": {},
            "changes": [{
                "bucket": "main",
                "collection": "quicksuggest",
                "last_modified": 0,
            }],
            "timestamp": 0,
            "backoff": null,
        }));
    });

    server.mock(|when, then| {
        when.method(GET)
            .path("/buckets/main/collections/quicksuggest/changeset");
        then.status(200).json_body(json!({
            "metadata": {},
            "changes": [],
            "timestamp": 0,
            "backoff": null,
        }));
    });
}
