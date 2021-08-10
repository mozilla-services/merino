//! Tests Merino's debug pages.
#![cfg(test)]

use crate::{merino_test_macro, TestingTools};
use reqwest::StatusCode;

#[merino_test_macro(|settings| settings.debug = false)]
async fn cant_use_debug_settings_route_when_debug_is_false(
    TestingTools { test_client, .. }: TestingTools,
) {
    let response = test_client
        .get("/debug/settings")
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.content_length(), Some(0));
}

#[merino_test_macro(|settings| settings.debug = true)]
async fn can_use_debug_settings_route_when_debug_is_true(
    TestingTools { test_client, .. }: TestingTools,
) {
    let response = test_client
        .get("/debug/settings")
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get("content-type").unwrap(),
        &"application/json"
    );
    assert!(response.json::<serde_json::Value>().await.is_ok());
}
