//! Tests Merino's ability to make basic suggestions.
#![cfg(test)]

use crate::{merino_test_macro, utils::test_tools::TestReqwestClient, TestingTools};
use anyhow::Result;
use httpmock::{Method::GET, MockServer};
use merino_settings::providers::{FixedConfig, RemoteSettingsConfig, SuggestionProviderConfig};
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
async fn suggest_adm_rs_works_empty(
    TestingTools {
        test_client,
        remote_settings_mock,
        ..
    }: TestingTools,
) -> Result<()> {
    setup_remote_settings_collection(&remote_settings_mock, &[]).await;

    let response = test_client.get("/api/v1/suggest?q=apple").send().await?;

    // Check that the status is 200 OK, and that the body is JSON. The
    // collection is empty so there shouldn't be any suggestion
    // response.
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await?;
    assert_eq!(body["suggestions"].as_array().unwrap().len(), 0);

    Ok(())
}

#[merino_test_macro(|settings| {
    settings.suggestion_providers.insert(
        "adm".to_string(),
        SuggestionProviderConfig::RemoteSettings(RemoteSettingsConfig::default())
    );
})]
async fn suggest_adm_rs_works_content(
    TestingTools {
        test_client,
        remote_settings_mock,
        ..
    }: TestingTools,
) -> Result<()> {
    setup_remote_settings_collection(&remote_settings_mock, &["apple", "banana"]).await;

    let response = test_client.get("/api/v1/suggest?q=apple").send().await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await?;
    assert_eq!(body["suggestions"].as_array().unwrap().len(), 1);
    assert_eq!(
        body["suggestions"][0]["title"].as_str().unwrap(),
        "Suggestion apple"
    );

    Ok(())
}

async fn setup_remote_settings_collection(server: &MockServer, suggestions: &[&str]) {
    let mut changes = suggestions
        .iter()
        .map(|s| {
            assert_ne!(*s, "icon");
            json!({
                "id": s,
                "type": "data",
                "last_modified": 0,
                "attachment": {
                    "location": format!("/attachment/data-{}.json", s),
                    "hash": s,
                }
            })
        })
        .collect::<Vec<_>>();
    changes.push(json!({
        "id": "icon-1",
        "type": "icon",
        "last_modified": 0,
        "attachment": {
            "location": "/attachment/icon-1.png",
            "hash": "icon"
        }
    }));

    server
        .mock_async(|when, then| {
            when.method(GET)
                .path("/v1/buckets/main/collections/quicksuggest/changeset");
            then.status(200).json_body(json!({
                "changes": changes,
            }));
        })
        .await;

    for (idx, s) in suggestions.iter().enumerate() {
        server
            .mock_async(|when, then| {
                when.method(GET)
                    .path(format!("/attachment/data-{}.json", s));
                then.status(200).json_body(json!([{
                    "id": idx,
                    "url": format!("https://example.com/#url/{}", s),
                    "click_url": format!("https://example.com/#click/{}", s),
                    "impression_url": format!("https://example.com/#impression/{}", s),
                    "iab_category": "5 - Education",
                    "icon": "1",
                    "advertiser": "fake",
                    "title": format!("Suggestion {}", s),
                    "keywords": [s],
                }]));
            })
            .await;
    }
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

#[merino_test_macro(|settings| {
    // FixedProvider is only allowed when debug is true.
    settings.debug = true;
    for color in ["red", "blue", "green"] {
        settings.suggestion_providers.insert(
            color.to_uppercase(),
            SuggestionProviderConfig::Fixed(FixedConfig { value: color.to_string() })
        );
    }
})]
async fn suggest_provider_id_is_listed(
    TestingTools { test_client, .. }: TestingTools,
) -> Result<()> {
    let response = test_client.get("/api/v1/suggest?q=test").send().await?;
    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await?;
    let returned_providers = body["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["provider"].as_str().unwrap())
        .collect::<HashSet<_>>();
    let expected_providers = ["RED", "BLUE", "GREEN"].iter().cloned().collect();
    assert_eq!(returned_providers, expected_providers);

    Ok(())
}

#[merino_test_macro(|settings| {
    // FixedProvider is only allowed when debug is true.
    settings.debug = true;
    for color in ["red", "blue", "green"] {
        settings.suggestion_providers.insert(
            color.to_string(),
            SuggestionProviderConfig::Fixed(FixedConfig { value: color.to_string() })
        );
    }
})]
async fn suggest_providers_can_be_filtered(
    TestingTools { test_client, .. }: TestingTools,
) -> Result<()> {
    async fn test(test_client: &TestReqwestClient, providers: Vec<&str>) -> Result<()> {
        let url = format!("/api/v1/suggest?q=test&providers={}", providers.join(","));
        let response = test_client.get(&url).send().await?;
        assert_eq!(response.status(), StatusCode::OK);

        let body: serde_json::Value = response.json().await?;
        let returned_providers = body["suggestions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["provider"].as_str().unwrap())
            .collect::<HashSet<_>>();
        let expected_providers = providers.into_iter().collect();
        assert_eq!(returned_providers, expected_providers);
        Ok(())
    }

    test(&test_client, vec!["red", "blue", "green"]).await?;
    test(&test_client, vec!["blue", "green"]).await?;
    test(&test_client, vec!["green", "red"]).await?;
    test(&test_client, vec!["blue"]).await?;
    test(&test_client, vec![]).await?;

    Ok(())
}

#[merino_test_macro(|settings| {
    // FixedProvider is only allowed when debug is true.
    settings.debug = true;
    for color in ["red", "blue", "green"] {
        settings.suggestion_providers.insert(
            color.to_string(),
            SuggestionProviderConfig::Fixed(FixedConfig { value: color.to_string() })
        );
    }
})]
async fn suggest_providers_are_resilient_to_unknown_providers(
    TestingTools { test_client, .. }: TestingTools,
) -> Result<()> {
    let url = "/api/v1/suggest?q=test&providers=red,blue,orange";
    let response = test_client.get(url).send().await?;
    assert_eq!(response.status(), StatusCode::OK);

    let body: serde_json::Value = response.json().await?;
    let returned_providers = body["suggestions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["provider"].as_str().unwrap())
        .collect::<HashSet<_>>();
    let expected_providers = ["red", "blue"].iter().cloned().collect();
    assert_eq!(returned_providers, expected_providers);

    Ok(())
}
