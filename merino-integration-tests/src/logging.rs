//! Tests that Merino logs behave as expected.
//!
//! This module should be used for general logging behavior. Logging behavior for
//! specific parts of Merino should be placed in more specific test modules.
#![cfg(test)]

use crate::{merino_test_macro, TestingTools};
use anyhow::Result;

#[merino_test_macro]
async fn error_handler_writes_logs(
    TestingTools {
        test_client,
        mut log_watcher,
        ..
    }: TestingTools,
) -> Result<()> {
    test_client
        .get("/__error__")
        .send()
        .await
        .expect("failed to execute request");

    assert!(log_watcher.has(|msg| msg.field_contains("message", "__error__")));

    Ok(())
}
