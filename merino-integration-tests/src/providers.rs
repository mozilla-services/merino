//! Tests Merino's ability to introspect providers by name and customize them.
#![cfg(test)]

use crate::{merino_test_macro, TestingTools};
use anyhow::Result;
use merino_settings::providers::{FixedConfig, SuggestionProviderConfig};
use pretty_assertions::assert_eq;
use reqwest::StatusCode;
use serde_json::json;

#[merino_test_macro(|settings| {
    // Fixed can only be used when debug is true.
    settings.debug = true;
    for color in ["red", "blue", "green"] {
        settings.suggestion_providers.insert(
            color.to_string(),
            SuggestionProviderConfig::Fixed(FixedConfig { value: color.to_string() })
        );
    }
})]
async fn providers_are_listed(TestingTools { test_client, .. }: TestingTools) -> Result<()> {
    let response = test_client.get("/api/v1/providers").send().await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.json::<serde_json::Value>().await?,
        json!({
            "providers": {
                "red": {"id": "red", "availability": "enabled_by_default"},
                "blue": {"id": "blue", "availability": "enabled_by_default"},
                "green": {"id": "green", "availability": "enabled_by_default"},
            }
        })
    );

    Ok(())
}

#[merino_test_macro(|settings| {
    // Fixed can only be used when debug is true.
    settings.debug = true;
    settings.suggestion_providers.insert(
        "real-provider".to_string(),
        SuggestionProviderConfig::Fixed(FixedConfig { value: "real-provider".to_string() })
    );
    settings.suggestion_providers.insert(
        "null-provider".to_string(),
        SuggestionProviderConfig::Null,
    );
})]
async fn null_providers_are_not_listed(
    TestingTools { test_client, .. }: TestingTools,
) -> Result<()> {
    let response = test_client.get("/api/v1/providers").send().await?;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.json::<serde_json::Value>().await?,
        json!({
            "providers": {
                "real-provider": {"id": "real-provider", "availability": "enabled_by_default"},
            }
        })
    );

    Ok(())
}
