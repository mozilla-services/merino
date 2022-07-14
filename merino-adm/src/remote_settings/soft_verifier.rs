// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! A custom signature verifier implementation that marks the verification
//! as successful even in case of problems, but logs a warning about the
//! failure.

use anyhow::Context;
use async_trait::async_trait;
use remote_settings_client::{
    client::net::Requester, Collection, RingVerifier, SignatureError, Verification,
};
use sentry_anyhow::capture_anyhow;

/// A custom signature verification helper to use with the
/// remote-settings-client library.
pub struct SoftVerifier {
    /// The wrapped RingVerifier.
    inner: RingVerifier,
}

impl SoftVerifier {
    /// Instantiate a new signature verifier.
    pub fn new() -> Self {
        SoftVerifier {
            inner: RingVerifier {},
        }
    }
}

#[async_trait]
impl Verification for SoftVerifier {
    fn verify_nist384p_chain(
        &self,
        epoch_seconds: u64,
        pem_bytes: &[u8],
        root_hash: &str,
        subject_cn: &str,
        message: &[u8],
        signature: &[u8],
    ) -> Result<(), SignatureError> {
        self.inner
            .verify_nist384p_chain(
                epoch_seconds,
                pem_bytes,
                root_hash,
                subject_cn,
                message,
                signature,
            )
            .context("nist384p verification for Remote Settings")
            .or_else(|e| {
                tracing::error!(
                    r#type = "adm.remote-settings.soft-verifier.verify_nist384p_chain",
                    "nist384p chain verification failed. {:#?}",
                    e
                );
                capture_anyhow(&e);
                Ok(())
            })
    }

    async fn verify(
        &self,
        requester: &'_ (dyn Requester + 'static),
        collection: &Collection,
        root_hash: &str,
    ) -> Result<(), SignatureError> {
        self.inner
            .verify(requester, collection, root_hash)
            .await
            .context("Signature verification for Remote Settings")
            .or_else(|e| {
                tracing::error!(
                    r#type = "adm.remote-settings.soft-verifier.verify",
                    "Verification failed. {:#?}",
                    e
                );
                capture_anyhow(&e);
                Ok(())
            })
    }

    fn verify_sha256_hash(&self, content: &[u8], expected: &[u8]) -> Result<(), SignatureError> {
        // We don't need to catch errors on this one: this method is not used
        // for signature verification, but just for verifying the integrity of
        // the downloaded attachments.
        self.inner.verify_sha256_hash(content, expected)
    }
}
