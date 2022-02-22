//! Tests reconfiguration of Merino providers.
#![cfg(test)]

use crate::{merino_test_macro, TestingTools};
use merino_settings::providers::{FixedConfig, RedisCacheConfig, SuggestionProviderConfig};
use redis::Commands;
use reqwest::StatusCode;
use serde_json::Value;
use std::collections::HashMap;
use std::time::Duration;

#[merino_test_macro(|settings| {
    settings.debug = true;
    settings.suggestion_providers.insert(
        "fixed_redis".to_string(),
        SuggestionProviderConfig::RedisCache(RedisCacheConfig::with_inner(
            SuggestionProviderConfig::Fixed(FixedConfig { value: "foo".to_owned() })
        )),
    );
})]
async fn test_reconfigure(
    TestingTools {
        test_client,
        mut redis_client,
        ..
    }: TestingTools,
) {
    let keys_before: Vec<String> = redis_client
        .keys("provider:v1:*")
        .expect("Could not get keys from redis");
    assert!(keys_before.is_empty());

    let response = test_client
        .get("/api/v1/suggest?q=foo")
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);

    // Prepare for updating the inner provider.
    // Reset the inner `FixedProvider` also reduce the `default_ttl`.
    let mut redis_config =
        RedisCacheConfig::with_inner(SuggestionProviderConfig::Fixed(FixedConfig {
            value: "bar".to_owned(),
        }));
    redis_config.default_ttl = Duration::from_secs(1);
    let new_config: HashMap<String, SuggestionProviderConfig> = [(
        "fixed_redis".to_owned(),
        SuggestionProviderConfig::RedisCache(redis_config),
    )]
    .into_iter()
    .collect();

    // Request for a reconfigure for Redis provider.
    // Note that this reconfigure endpoint is only exposed in debug mode.
    let response = test_client
        .post("/api/v1/providers/reconfigure")
        .json(&new_config)
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let response = test_client
        .get("/api/v1/suggest?q=bar")
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);

    // Verify the new `FixedProvider` is in use.
    let http_response: Value = response.json().await.expect("response was not json");
    let http_suggestions = http_response["suggestions"].as_array().unwrap();
    assert_eq!(http_suggestions.len(), 1);
    assert_eq!(http_suggestions[0]["title"], "bar");

    // Wait for a bit to let the newly inserted value expire.
    tokio::time::sleep(Duration::from_millis(1500)).await;

    // "bar" should not be in the cache as its key already expires.
    let keys_after: Vec<String> = redis_client
        .keys("provider:v1:*")
        .expect("Could not get keys");
    assert_eq!(
        keys_after.len(),
        1,
        "Only one item should be left in the cache"
    );

    let encoded: String = redis_client
        .get(&keys_after[0])
        .expect("Could not get cached item");

    let cache_suggestions: Vec<Value> =
        serde_json::from_str(&encoded[2..]).expect("Couldn't parse cached item");
    // The cached value should be the old one "foo".
    assert_eq!(cache_suggestions[0]["title"], "foo");
}
