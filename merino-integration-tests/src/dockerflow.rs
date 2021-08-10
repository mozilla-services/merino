//! Tests that Merino conforms to [Dockerflow](https://github.com/mozilla-services/dockerflow).
#![cfg(test)]

use crate::{merino_test_macro, TestingTools};
use anyhow::Result;
use reqwest::StatusCode;
use serde::Deserialize;

#[merino_test_macro]
async fn lbheartbeat_works(TestingTools { test_client, .. }: TestingTools) {
    let response = test_client
        .get("/__lbheartbeat__")
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.content_length(), Some(0));
}

#[merino_test_macro]
async fn heartbeat_works(TestingTools { test_client, .. }: TestingTools) -> Result<()> {
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

#[merino_test_macro]
async fn version_works(TestingTools { test_client, .. }: TestingTools) -> Result<()> {
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

#[merino_test_macro]
async fn error_works(TestingTools { test_client, .. }: TestingTools) -> Result<()> {
    let response = test_client
        .get("/__error__")
        .send()
        .await
        .expect("failed to execute request");

    assert!(response.status().is_server_error());

    Ok(())
}
