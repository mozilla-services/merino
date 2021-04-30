//! Tests Merino's debug pages.

use reqwest::StatusCode;

use crate::TestingTools;

#[actix_rt::test]
async fn cant_use_debug_settings_route_when_debug_is_false() {
    let TestingTools { test_client, .. } = TestingTools::new(|settings| settings.debug = false);

    let response = test_client
        .get("/debug/settings")
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.content_length(), Some(0));
}

#[actix_rt::test]
async fn can_use_debug_settings_route_when_debug_is_true() {
    let TestingTools { test_client, .. } = TestingTools::new(|settings| {
        settings.debug = true;
    });

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
