//! Tests Merino's debug pages.
#![cfg(test)]

use crate::{merino_test, TestingTools};
use merino_settings::Settings;
use reqwest::StatusCode;

#[actix_rt::test]
async fn cant_use_debug_settings_route_when_debug_is_false() {
    merino_test(
        |settings: &mut Settings| settings.debug = false,
        |TestingTools { test_client, .. }| async move {
            let response = test_client
                .get("/debug/settings")
                .send()
                .await
                .expect("failed to execute request");

            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            assert_eq!(response.content_length(), Some(0));
        },
    )
    .await
}

#[actix_rt::test]
async fn can_use_debug_settings_route_when_debug_is_true() {
    merino_test(
        |settings: &mut Settings| settings.debug = true,
        |TestingTools { test_client, .. }| async move {
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
        },
    )
    .await
}
