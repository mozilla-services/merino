// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! An HTTP client implementation to use with the remote-settings-client.

use anyhow::{Context, Error};
use async_trait::async_trait;
use remote_settings_client::client::net::{
    Headers as RsHeaders, Method as RsMethod, Requester as RsRequester, Response as RsResponse,
    Url as RsUrl,
};
use reqwest::{header::CONTENT_TYPE, Method, Response};
use std::time::Duration;

/// An remote-settings-client HTTP client that uses Reqwest.
#[derive(Debug)]
pub struct ReqwestClient {
    /// The client that will be used to make http requests.
    reqwest_client: reqwest::Client,
    /// The HTTP timeout in seconds.
    http_timeout: Duration,
}

impl ReqwestClient {
    /// Instantiate a new Reqwest client to perform HTTP requests.
    pub fn try_new(http_timeout: Duration) -> Result<ReqwestClient, Error> {
        let reqwest_client = reqwest::Client::builder()
            // Disable the connection pool to avoid the IncompleteMessage errors.
            // See #259 for more details.
            .pool_max_idle_per_host(0)
            .build()?;
        Ok(Self {
            reqwest_client,
            http_timeout,
        })
    }
}

#[async_trait]
impl RsRequester for ReqwestClient {
    async fn get(&self, url: RsUrl) -> Result<RsResponse, ()> {
        self.request_json(RsMethod::GET, url, vec![], RsHeaders::default())
            .await
    }

    async fn request_json(
        &self,
        method: RsMethod,
        url: RsUrl,
        data: Vec<u8>,
        headers: RsHeaders,
    ) -> Result<RsResponse, ()> {
        let method = match method {
            RsMethod::GET => Method::GET,
            RsMethod::PATCH => Method::PATCH,
            RsMethod::POST => Method::POST,
            RsMethod::PUT => Method::PUT,
            RsMethod::DELETE => Method::DELETE,
        };
        let headers = (&headers).try_into().map_err(|e| {
            tracing::error!(
                r#type = "adm.remote-settings.reqwest.headers-conversion-failed",
                "ReqwestClient - unable to try_into headers. {:#?}",
                e
            );
        })?;

        match self
            .reqwest_client
            .request(method.clone(), url.clone())
            .header(CONTENT_TYPE, "application/json")
            .headers(headers)
            .body(data)
            .timeout(self.http_timeout)
            .send()
            .await
            .and_then(Response::error_for_status)
            .context(format!(
                "Performing HTTP request for Remote Settings: {}",
                url
            )) {
            Err(e) => {
                tracing::error!(
                    r#type = "adm.remote-settings.reqwest.get-failed",
                    "ReqwestClient - unable to submit {} request. {:#?}",
                    method,
                    e
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
                        r#type = "adm.remote-settings.reqwest.parsing-failed",
                        "ReqwestClient - unable to parse response body. {:#?}",
                        err
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
