// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! An HTTP client implementation to use with the remote-settings-client.

use anyhow::Context;
use async_trait::async_trait;
use remote_settings_client::client::net::{
    Headers as RsHeaders, Requester as RsRequester, Response as RsResponse, Url as RsUrl,
};
use reqwest::Response;

/// An remote-settings-client HTTP client that uses Reqwest.
#[derive(Debug)]
pub struct ReqwestClient {
    /// The client that will be used to make http requests.
    reqwest_client: reqwest::Client,
}

impl ReqwestClient {
    /// Instantiate a new Reqwest client to perform HTTP requests.
    pub fn new() -> ReqwestClient {
        Self {
            reqwest_client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl RsRequester for ReqwestClient {
    async fn get(&self, url: RsUrl) -> Result<RsResponse, ()> {
        match self
            .reqwest_client
            .get(url.clone())
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await
            .and_then(Response::error_for_status)
            .context(format!(
                "Performing HTTP request for Remote Settings: {}",
                url
            )) {
            Err(e) => {
                tracing::error!(
                    "ReqwestClient - unable to submit GET request. {:?}",
                    e.to_string()
                );
                Err(())
            }
            Ok(response) => {
                let status = response.status().as_u16();
                let mut headers: RsHeaders = RsHeaders::new();
                for h in response.headers() {
                    headers
                        .entry(h.0.to_string())
                        .or_insert_with(|| h.1.to_str().unwrap_or_default().to_string());
                }

                let body = response.bytes().await.map_err(|err| {
                    tracing::error!(
                        "ReqwestClient - unable to parse response body. {:?}",
                        err.to_string()
                    );
                })?;

                Ok(RsResponse {
                    status,
                    body: body.to_vec(),
                    headers,
                })
            }
        }
    }
}
