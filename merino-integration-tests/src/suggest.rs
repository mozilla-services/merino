//! Tests Merino's ability to make basic suggestions.
#![cfg(test)]

use crate::{merino_test_macro, utils::test_tools::TestReqwestClient, TestingTools};
use anyhow::Result;
use lazy_static::lazy_static;
use merino_settings::providers::{
    FixedConfig, KeywordFilterConfig, MultiplexerConfig, RemoteSettingsConfig,
    SuggestionProviderConfig,
};
use reqwest::StatusCode;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use tracing::Level;

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
async fn test_returns_request_id(TestingTools { test_client, .. }: TestingTools) -> Result<()> {
    let response = test_client
        .get("/api/v1/suggest?q=apple&client_variants=one,two")
        .send()
        .await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await?;
    assert!(
        body.as_object().unwrap().contains_key("request_id"),
        "response should have a request_id"
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
    settings.remote_settings.test_changes = Some(vec![])
})]
async fn suggest_adm_rs_works_empty(TestingTools { test_client, .. }: TestingTools) -> Result<()> {
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
    settings.remote_settings.test_changes = Some(vec!["apple".to_string(), "banana".to_string()]);
})]
async fn suggest_adm_rs_works_content(
    TestingTools { test_client, .. }: TestingTools,
) -> Result<()> {
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

lazy_static! {
    static ref TEST_SCORE: f64 = std::f64::consts::TAU.fract();
}

#[merino_test_macro(|settings| {
    settings.suggestion_providers.insert(
        "adm".to_string(),
        SuggestionProviderConfig::RemoteSettings(RemoteSettingsConfig {
            suggestion_score: *TEST_SCORE as f32,
            ..RemoteSettingsConfig::default()
        })
    );
    settings.remote_settings.test_changes = Some(vec!["apple".to_string()]);
})]
async fn suggest_adm_rs_score_is_configurable(
    TestingTools { test_client, .. }: TestingTools,
) -> Result<()> {
    let response = test_client.get("/api/v1/suggest?q=apple").send().await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await?;
    // Check that the score is approximately correct.
    assert!((body["suggestions"][0]["score"].as_f64().unwrap() - *TEST_SCORE).abs() < 0.001);

    Ok(())
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

#[merino_test_macro(|settings| {
    // FixedProvider is only allowed when debug is true.
    settings.debug = true;
    let mut blocklist = HashMap::new();
    blocklist.insert("no-apple".to_string(), "(a|A)pple".to_string());

    let multiplexer = Box::new(SuggestionProviderConfig::Multiplexer(MultiplexerConfig{
        providers: vec![
            SuggestionProviderConfig::Fixed(FixedConfig { value: "apple".to_string() }),
            SuggestionProviderConfig::Fixed(FixedConfig { value: "anvil".to_string() }),
            SuggestionProviderConfig::Fixed(FixedConfig { value: "heavymetal".to_string() }),
        ]
    }));

    settings.suggestion_providers.insert("keyword_filter".to_string(), SuggestionProviderConfig::KeywordFilter(KeywordFilterConfig {
        suggestion_blocklist: blocklist,
        inner: multiplexer,
    }));
})]
async fn suggest_keywordfilter_works(
    TestingTools {
        test_client,
        mut metrics_watcher,
        ..
    }: TestingTools,
) -> Result<()> {
    let response = test_client.get("/api/v1/suggest?q=apple").send().await?;

    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value = response.json().await?;
    assert_eq!(body["suggestions"].as_array().unwrap().len(), 2);
    assert_eq!(body["suggestions"][0]["title"], json!("heavymetal"));
    assert_eq!(body["suggestions"][1]["title"], json!("anvil"));

    assert!(metrics_watcher.has_incr("keywordfilter.match"));

    Ok(())
}

#[merino_test_macro(|settings| {
    // This test is valid even with no providers specified.
    settings.suggestion_providers = HashMap::new();
    settings.log_full_request = true;
})]
async fn suggest_logs_searches_when_requested(
    TestingTools {
        mut log_watcher,
        test_client,
        ..
    }: TestingTools,
) -> Result<()> {
    // Just a long random value, nothing special.
    let query = "eec7c3d8-3bf6-11ec-a29b-bbdf015cc865";
    test_client
        .get(&format!("/api/v1/suggest?q={}", query))
        .send()
        .await?;

    assert!(log_watcher.has(|event| {
        event.level == Level::INFO
            && event.fields.get("r#type").and_then(Value::as_str) == Some("web.suggest.request")
            && event.fields.get("query").and_then(Value::as_str) == Some(query)
    }));

    // Only log lines tagged with the specified type contain the query
    for event in log_watcher.events() {
        // Levels above INFO aren't included in production logging
        if event.level == Level::DEBUG || event.level == Level::TRACE {
            continue;
        }
        // If the type is correct, the `has` assertion above ensures everything is ok
        if event.fields.get("r#type") == Some(&json!("web.suggest.request")) {
            assert_eq!(event.fields.get("sensitive"), Some(&json!(true)));
            continue;
        }
        // If this is a non-request, non-debug/trace log, then it should not have the query anywhere in it
        let text = serde_json::to_string(&event)?;
        assert!(!text.contains(query));
    }

    Ok(())
}

#[merino_test_macro(|settings| {
    // This test is valid even with no providers specified.
    settings.suggestion_providers = HashMap::new();
    settings.log_full_request = true;
})]
async fn suggest_logs_includes_session_id_and_seq_num_when_provided_in_query(
    TestingTools {
        mut log_watcher,
        test_client,
        ..
    }: TestingTools,
) -> Result<()> {
    // Just a long random value, nothing special.
    let session_id = "deadbeef-0000-1111-2222-333344445555";
    let sequence_no = 11;
    let query = "hello_test_query";
    test_client
        .get(&format!(
            "/api/v1/suggest?q={}&sid={}&seq={}",
            query, session_id, sequence_no
        ))
        .send()
        .await?;

    assert!(log_watcher.has(|event| {
        event.level == Level::INFO
            && event.fields.get("r#type").and_then(Value::as_str) == Some("web.suggest.request")
            && event.fields.get("query").and_then(Value::as_str) == Some(query)
    }));

    // Only log lines tagged with the specified type contain the query
    for event in log_watcher.events() {
        // Levels above INFO aren't included in production logging
        if event.level == Level::DEBUG || event.level == Level::TRACE {
            continue;
        }
        // If the type is correct, the `has` assertion above ensures everything is ok
        if event.fields.get("r#type") == Some(&json!("web.suggest.request")) {
            assert_eq!(
                event.fields.get("session_id").and_then(Value::as_str),
                Some(session_id)
            );
            assert_eq!(
                event.fields.get("sequence_no").and_then(Value::as_i64),
                Some(sequence_no)
            );
            continue;
        }
    }
    Ok(())
}

#[merino_test_macro(|settings| {
    // This test is valid even with no providers specified.
    settings.suggestion_providers = HashMap::new();
    settings.log_full_request = true;
})]
async fn suggest_logs_includes_session_id_and_seq_num_are_none_when_not_provided_in_query(
    TestingTools {
        mut log_watcher,
        test_client,
        ..
    }: TestingTools,
) -> Result<()> {
    // Just a long random value, nothing special.
    let query = "apple_test_query";

    test_client
        .get(&format!("/api/v1/suggest?q={}", query))
        .send()
        .await?;

    assert!(log_watcher.has(|event| {
        event.level == Level::INFO
            && event.fields.get("r#type").and_then(Value::as_str) == Some("web.suggest.request")
            && event.fields.get("query").and_then(Value::as_str) == Some(query)
    }));

    // Only log lines tagged with the specified type contain the query
    for event in log_watcher.events() {
        // Levels above INFO aren't included in production logging
        if event.level == Level::DEBUG || event.level == Level::TRACE {
            continue;
        }
        // If the type is correct, the `has` assertion above ensures everything is ok
        if event.fields.get("r#type") == Some(&json!("web.suggest.request")) {
            assert!(event.fields.get("session_id").is_none());
            assert!(event.fields.get("sequence_no").is_none());
            continue;
        }
    }
    Ok(())
}
#[merino_test_macro(|settings| {
    // This test is valid even with no providers specified.
    settings.suggestion_providers = HashMap::new();
    settings.log_full_request = false;
})]
async fn suggest_redacts_queries_when_requested(
    TestingTools {
        mut log_watcher,
        test_client,
        ..
    }: TestingTools,
) -> Result<()> {
    // Just a long random value, nothing special.
    let query = "eec7c3d8-3bf6-11ec-a29b-bbdf015cc865";
    test_client
        .get(&format!("/api/v1/suggest?q={}", query))
        .send()
        .await?;

    // Only log lines tagged with the specified type contain the query
    for event in log_watcher.events() {
        // Levels above INFO aren't included in production logging
        if event.level == Level::DEBUG || event.level == Level::TRACE {
            continue;
        }
        // If this is a non-debug/trace log, then it should not have the query anywhere in it
        let text = serde_json::to_string(&event)?;
        assert!(!text.contains(query));
    }

    Ok(())
}
