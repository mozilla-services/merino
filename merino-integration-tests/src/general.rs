//! Tests Merino's ability to make basic suggestions.
#![cfg(test)]

use crate::{merino_test_macro, TestingTools};
use anyhow::Result;
use reqwest::{header::HeaderValue, StatusCode};

#[merino_test_macro(|settings| settings.public_documentation = Some("https://example.com/".parse().unwrap()))]
async fn root_of_services_provides_public_docs(
    TestingTools { test_client, .. }: TestingTools,
) -> Result<()> {
    let response = test_client.get("/").send().await?;

    assert_eq!(response.status(), StatusCode::FOUND);
    assert_eq!(
        response.headers().get("location"),
        Some(&HeaderValue::from_static("https://example.com/"))
    );

    Ok(())
}

#[merino_test_macro(|settings| settings.public_documentation = None)]
async fn root_of_services_has_a_fallback_message(
    TestingTools { test_client, .. }: TestingTools,
) -> Result<()> {
    let response = test_client.get("/").send().await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.text().await?,
        "Merino is a Mozilla service providing information to the Firefox Suggest feature."
    );

    Ok(())
}
