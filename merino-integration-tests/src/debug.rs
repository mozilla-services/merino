//! Tests Merino's debug pages.

use reqwest::StatusCode;

use crate::start_app_server;

#[actix_rt::test]
async fn cant_use_debug_settings_route_when_debug_is_false() {
    let address = start_app_server(|settings| settings.debug = false);
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/debug/settings", &address))
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(response.content_length(), Some(0));
}

#[actix_rt::test]
async fn can_use_debug_settings_route_when_debug_is_true() {
    let address = start_app_server(|settings| {
        settings.debug = true;
    });
    let client = reqwest::Client::new();

    let response = client
        .get(&format!("{}/debug/settings", &address))
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
