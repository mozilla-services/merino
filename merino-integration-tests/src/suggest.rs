//! Tests Merino's ability to make basic suggestions.
#![cfg(test)]

use crate::{merino_test_macro, TestingTools};
use anyhow::Result;
use httpmock::{Method::GET, MockServer};
use merino_settings::providers::{RemoteSettingsConfig, SuggestionProviderConfig};
use reqwest::StatusCode;
use serde_json::json;
use std::collections::HashSet;

#[merino_test_macro(|settings| {
    // Wiki fruit is only enabled when debug is true.
    settings.debug = true;
    settings.suggestion_providers.insert("wiki_fruit".to_string(), SuggestionProviderConfig::WikiFruit);
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

#[merino_test_macro(|settings| {
    // Wiki fruit is only enabled when debug is true.
    settings.debug = true;
    settings.suggestion_providers.insert("wiki_fruit".to_string(), SuggestionProviderConfig::WikiFruit);
})]
async fn test_expected_suggestion_fields(
    TestingTools { test_client, .. }: TestingTools,
) -> Result<()> {
    let response = test_client.get("/api/v1/suggest?q=apple").send().await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await?;
    let keys: HashSet<_> = body["suggestions"][0].as_object().unwrap().keys().collect();

    let expected_keys = vec![
        "block_id",
        "full_keyword",
        "title",
        "url",
        "impression_url",
        "click_url",
        "provider",
        "advertiser",
        "is_sponsored",
        "icon",
        "score",
    ];
    dbg!(&keys);
    for expected_key in expected_keys {
        assert!(
            keys.contains(&expected_key.to_string()),
            "key {} should be included in suggestion objects",
            expected_key
        );
    }

    Ok(())
}

#[merino_test_macro]
async fn test_returns_client_variants(
    TestingTools { test_client, .. }: TestingTools,
) -> Result<()> {
    let response = test_client
        .get("/api/v1/suggest?q=apple&client_variants=one,two")
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await?;
    assert_eq!(
        *body["client_variants"].as_array().unwrap(),
        vec!["one", "two"]
    );

    Ok(())
}

#[merino_test_macro]
async fn test_expected_variant_fields(
    TestingTools { test_client, .. }: TestingTools,
) -> Result<()> {
    let response = test_client.get("/api/v1/suggest?q=apple").send().await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await?;
    assert!(
        body.as_object().unwrap().contains_key("client_variants"),
        "response should have a clients variants key"
    );
    assert!(
        body.as_object().unwrap().contains_key("server_variants"),
        "response should have a server variants key"
    );

    Ok(())
}

#[merino_test_macro(|settings| {
    settings.suggestion_providers.insert(
        "adm".to_string(),
        SuggestionProviderConfig::RemoteSettings(RemoteSettingsConfig::default())
    );
})]
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

#[merino_test_macro(|settings| {
    // Wiki fruit is only enabled when debug is true.
    settings.debug = true;
    settings.suggestion_providers.insert("wiki_fruit".to_string(), SuggestionProviderConfig::WikiFruit);
})]
async fn suggest_records_suggestion_metrics(
    TestingTools {
        test_client,
        mut metrics_watcher,
        ..
    }: TestingTools,
) -> Result<()> {
    let response = test_client.get("/api/v1/suggest?q=apple").send().await?;
    let body: serde_json::Value = response.json().await?;
    let response_suggestion_count = body["suggestions"].as_array().unwrap().len() as f64;
    assert!(metrics_watcher.has_histogram("request.suggestion-per", response_suggestion_count));
    Ok(())
}

#[merino_test_macro]
async fn suggest_records_client_variants_metrics(
    TestingTools {
        test_client,
        mut metrics_watcher,
        ..
    }: TestingTools,
) -> Result<()> {
    test_client
        .get("/api/v1/suggest?q=apple&client_variants=one,two")
        .send()
        .await?;

    assert!(metrics_watcher.has_incr("client_variants.one"));
    assert!(metrics_watcher.has_incr("client_variants.two"));
    Ok(())
}
