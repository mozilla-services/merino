//! Tests Merino's caching system.
#![cfg(test)]

use crate::{merino_test_macro, TestingTools};
use merino_settings::CacheType;
use redis::Commands;
use reqwest::{header::HeaderValue, StatusCode};
use serde_json::Value;
use std::time::Duration;

#[merino_test_macro(|settings| {
    settings.debug = true;
    settings.providers.wiki_fruit.enabled = true;
    settings.providers.wiki_fruit.cache = CacheType::Redis;
    settings.redis_cache.default_ttl = Duration::from_secs(600);
})]
async fn responses_are_stored_in_the_cache(
    TestingTools {
        test_client,
        redis_client,
        ..
    }: TestingTools,
) {
    let mut redis_client = redis_client.expect("This test requires a Redis connection");
    let keys_before: Vec<String> = redis_client
        .keys("*")
        .expect("Could not get keys from redis");
    assert!(keys_before.is_empty());

    let response = test_client
        .get("/api/v1/suggest?q=apple")
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);
    let http_response: Value = response.json().await.expect("response was not json");
    let http_suggestions = http_response["suggestions"].as_array();

    tokio::time::sleep(Duration::from_millis(1000)).await;

    let keys_after: Vec<String> = redis_client.keys("*").expect("Could not get keys");
    assert_eq!(keys_after.len(), 1, "an item should be in the cache");

    let encoded: String = redis_client
        .get(&keys_after[0])
        .expect("Could not get cached item");
    assert_eq!(&encoded[0..2], "v0", "version tag is included");
    let cache_suggestions: Vec<Value> =
        serde_json::from_str(&encoded[2..]).expect("Couldn't parse cached item");
    assert_eq!(Some(&cache_suggestions), http_suggestions);
}

#[merino_test_macro(|settings| {
    settings.debug = true;
    settings.providers.wiki_fruit.enabled = true;
    settings.providers.wiki_fruit.cache = CacheType::Redis;
    settings.redis_cache.default_ttl = Duration::from_secs(600);
})]
async fn bad_cache_data_is_handled(
    TestingTools {
        test_client,
        redis_client,
        mut log_watcher,
        ..
    }: TestingTools,
) {
    let mut redis_client = redis_client.expect("This test requires a Redis connection");
    let url = "/api/v1/suggest?q=apple";

    // one request to prime the cache
    let response = test_client
        .get(url)
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);

    let keys: Vec<String> = redis_client.keys("*").expect("Could not get keys");
    let key = keys.into_iter().next().unwrap();

    // Mess with the cache, to cause an error
    let _: () = redis::Cmd::set(&key, 42)
        .query(&mut redis_client)
        .expect("Couldn't write to cache");

    // Another request which should attempt, and fail, to read from the cache.
    let response = test_client
        .get(url)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("x-cache"),
        Some(&HeaderValue::from_static("error")),
    );
    // Check that the expected error type was reported in the logs.
    assert!(log_watcher.has(|event| {
        event.field_contains("message", "not of expected type")
            && matches!(
                event.fields.get("key"),
                Some(serde_json::Value::String(k)) if *k == key
            )
    }));

    // TODO the bad cache entry should be deleted, but this is handled
    // as a background task. How do we wait for that?
}

#[merino_test_macro(|settings| {
    settings.debug = true;
    settings.providers.wiki_fruit.enabled = true;
    settings.providers.wiki_fruit.cache = CacheType::Redis;
    settings.redis_cache.default_ttl = Duration::from_secs(600);
})]
async fn missing_ttls_are_re_set(
    TestingTools {
        test_client,
        redis_client,
        mut log_watcher,
        ..
    }: TestingTools,
) {
    let mut redis_client = redis_client.expect("This test requires a Redis connection");
    let url = "/api/v1/suggest?q=apple";

    // one request to prime the cache
    let response = test_client
        .get(url)
        .send()
        .await
        .expect("failed to execute request");
    assert_eq!(response.status(), StatusCode::OK);

    let keys: Vec<String> = redis_client.keys("*").expect("Could not get keys");
    let key = keys.into_iter().next().unwrap();

    // Remove the TTL from the cached item
    let _: () = redis::Cmd::persist(&key)
        .query(&mut redis_client)
        .expect("Couldn't write to cache");

    // Another request which should succeed.
    let response = test_client
        .get(url)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("x-cache"),
        Some(&HeaderValue::from_static("hit")),
    );
    // Check that the expected error type was reported in the logs.
    log_watcher.has(|event| {
        event.field_contains("message", "not of expected type")
            && matches!(
                event.fields.get("key"),
                Some(serde_json::Value::String(k)) if *k == key
            )
    });
}
