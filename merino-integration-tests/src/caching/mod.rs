//! Tests that apply to all of Merino's caching systems.
#![cfg(test)]

mod redis_tests;

use crate::{merino_test_macro, TestingTools};
use merino_settings::CacheType;
use parameterized::parameterized;
use reqwest::{header::HeaderValue, StatusCode};
use std::time::Duration;

#[merino_test_macro(|settings, cache: CacheType| {
    settings.debug = true;
    settings.providers.wiki_fruit.enabled = true;
    settings.providers.wiki_fruit.cache = cache;
    settings.redis_cache.default_ttl = Duration::from_secs(600);
    settings.memory_cache.default_ttl = Duration::from_secs(600);
})]
#[parameterized(cache = { CacheType::Redis, CacheType::Memory })]
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
