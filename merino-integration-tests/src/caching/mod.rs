//! Tests that apply to all of Merino's caching systems.
#![cfg(test)]

mod redis_tests;

use crate::{merino_test_macro, TestingTools};
use merino_settings::providers::{MemoryCacheConfig, RedisCacheConfig, SuggestionProviderConfig};
use parameterized::parameterized;
use reqwest::{header::HeaderValue, StatusCode};

#[merino_test_macro(|settings, cache: &str| {
    settings.debug = true;
    let wiki_fruit = SuggestionProviderConfig::WikiFruit;

    match cache {
        "redis" => settings.suggestion_providers.insert(
            "wiki_fruit_redis".to_string(),
            SuggestionProviderConfig::RedisCache(RedisCacheConfig::with_inner(wiki_fruit)),
        ),
        "memory" => settings.suggestion_providers.insert(
            "wiki_fruit_memory".to_string(),
            SuggestionProviderConfig::MemoryCache(MemoryCacheConfig::with_inner(wiki_fruit)),
        ),
        _ => panic!("unexpected cache {}", cache),
    };
})]
#[parameterized(cache = { "redis", "memory" })]
async fn cache_status_is_reported(TestingTools { test_client, .. }: TestingTools) {
    let url = "/api/v1/suggest?q=apple";

    // one request to prime the cache
    let response = test_client
        .get(url)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("x-cache"),
        Some(&HeaderValue::from_static("miss")),
    );

    // And another request which should come from the cache
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
}
