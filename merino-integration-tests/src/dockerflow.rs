//! Tests that Merino conforms to [Dockerflow](https://github.com/mozilla-services/dockerflow).

use anyhow::Result;
use reqwest::StatusCode;
use serde::Deserialize;

use crate::start_app_server;

#[actix_rt::test]
async fn lbheartbeat_works() {
    let test_client = start_app_server(|_| ());

    let response = test_client
        .get("/__lbheartbeat__")
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.content_length(), Some(0));
}

#[actix_rt::test]
async fn heartbeat_works() -> Result<()> {
    let test_client = start_app_server(|_| ());

    let response = test_client
        .get("/__heartbeat__")
        .send()
        .await
        .expect("failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(
        response
            .headers()
            .get_all("content-type")
            .iter()
            .collect::<Vec<_>>(),
        vec!["application/json"]
    );
    Ok(())
}

#[actix_rt::test]
async fn version_works() -> Result<()> {
    let test_client = start_app_server(|_| ());

    let response = test_client
        .get("/__version__")
        .send()
        .await
        .expect("failed to execute request");

    assert!(response.status().is_success());
    assert_eq!(
        response
            .headers()
            .get_all("content-type")
            .iter()
            .collect::<Vec<_>>(),
        vec!["application/json"]
    );

    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct VersionInfo {
        source: String,
        version: String,
        commit: String,
        build: String,
    }
    let body: Result<VersionInfo, _> = response.json().await;
    assert!(body.is_ok());

    Ok(())
}

#[actix_rt::test]
async fn error_works() -> Result<()> {
    let test_client = start_app_server(|_| ());

    let response = test_client
        .get("/__error__")
        .send()
        .await
        .expect("failed to execute request");

    assert!(response.status().is_server_error());

    Ok(())
}
