//! Tests Merino's ability to make basic suggestions.
#![cfg(test)]

use crate::{merino_test_macro, TestingTools};
use anyhow::Result;
use httpmock::{Method::GET, MockServer};
use reqwest::StatusCode;
use serde_json::json;

#[merino_test_macro(|settings| {
    // Wiki fruit is only enabled when debug is true.
    settings.debug = true;
    settings.providers.wiki_fruit.enabled = true;
})]
async fn suggest_wikifruit_works(TestingTools { test_client, .. }: TestingTools) -> Result<()> {
    let response = test_client.get("/api/v1/suggest?q=apple").send().await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await?;
    assert_eq!(
        body["suggestions"][0]["url"],
        json!("https://en.wikipedia.org/wiki/Apple")
    );

    Ok(())
}

#[merino_test_macro(|settings| settings.providers.adm_rs.enabled = true )]
async fn suggest_adm_rs_works(
    TestingTools {
        test_client,
        remote_settings_mock,
        ..
    }: TestingTools,
) -> Result<()> {
    setup_empty_remote_settings_collection(remote_settings_mock);

    let response = test_client.get("/api/v1/suggest?q=apple").send().await?;

    // Check that the status is 200 OK, and that the body is JSON. The
    // collection is empty so there shouldn't be any suggestion
    // response.
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await?;
    assert_eq!(body["suggestions"].as_array().unwrap().len(), 0);

    Ok(())
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
