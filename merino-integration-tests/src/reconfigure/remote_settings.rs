//! Tests reconfiguration of Merino providers.
#![cfg(test)]

use crate::{merino_test_macro, TestingTools};
use merino_settings::providers::{RemoteSettingsConfig, SuggestionProviderConfig};
use reqwest::StatusCode;
use std::collections::HashMap;
use std::time::Duration;

#[merino_test_macro(|settings| {
    settings.debug = true;
    settings.suggestion_providers.insert(
        "adm-remote-settings".to_owned(),
        SuggestionProviderConfig::RemoteSettings(RemoteSettingsConfig::default()),
    );
})]
async fn test_reconfigure(TestingTools { test_client, .. }: TestingTools) {
    let response = test_client
        .get("/api/v1/suggest?q=foo")
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);

    let new_config: HashMap<String, SuggestionProviderConfig> = [(
        "adm-remote-settings".to_owned(),
        SuggestionProviderConfig::RemoteSettings(RemoteSettingsConfig {
            bucket: None,
            collection: None,
            resync_interval: Duration::from_secs(600),
            suggestion_score: 0.4_f32,
        }),
    )]
    .into_iter()
    .collect();

    // Request for a reconfigure for Remote Settings provider.
    // Note that this reconfigure endpoint is only exposed in debug mode.
    let response = test_client
        .post("/api/v1/providers/reconfigure")
        .json(&new_config)
        .send()
        .await
        .expect("failed to execute request");

    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}
