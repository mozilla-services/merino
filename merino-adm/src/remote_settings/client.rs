//! A remote settings client

use anyhow::{anyhow, Context};
use http::HeaderValue;
use merino_suggest::SetupError;
use reqwest::{Response, Url};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{hash_map, HashMap},
    fmt::Debug,
    sync::{Arc, RwLock},
};

/// A client to interact with a remote settings collection.
pub struct RemoteSettingsClient {
    /// Server URL
    server_url: Url,
    /// Bucket ID
    bucket_id: String,
    /// Collection ID
    collection_id: String,
    /// The records we've seen already.
    records: HashMap<String, Record>,
    /// The attachment objects we've seen already.
    attachments: HashMap<String, Arc<LazyAttachment>>,
    /// The base to download attachments from.
    attachment_base_url: Option<Url>,
    /// The client that will be used to make http requests.
    reqwest_client: reqwest::Client,
}

impl RemoteSettingsClient {
    /// Make a new client targeting the target collection information.
    pub fn new(
        server_url: &str,
        bucket_id: String,
        collection_id: String,
    ) -> Result<Self, SetupError> {
        let server_url_parsed = Url::parse(server_url)
            .context(format!("Building remote settings URL: {}", server_url))
            .map_err(SetupError::InvalidConfiguration)?;

        Ok(Self {
            server_url: server_url_parsed,
            bucket_id,
            collection_id,
            attachment_base_url: None,

            records: HashMap::new(),
            attachments: HashMap::new(),
            reqwest_client: reqwest::Client::new(),
        })
    }

    /// Fetch changes from the server since the last sync.
    pub async fn sync(&mut self) -> Result<(), SetupError> {
        let last_modified = self.records().map(|r| r.last_modified).max().unwrap_or(0);

        let mut records_url = self
            .server_url
            .join(&format!(
                "v1/buckets/{}/collections/{}/changeset",
                self.bucket_id, self.collection_id
            ))
            .context("Building changeset URL")
            .map_err(SetupError::InvalidConfiguration)?;
        records_url.query_pairs_mut().extend_pairs(&[
            /* As of 2021-10-14, `_expected` is a [required parameter][0] for the
             * changeset endpoint, but its value [doesn't matter][1].
             *
             * [0]: https://github.com/Kinto/kinto-changes/blob/caa43cca8d0e747756e2cf771f209e3636617663/kinto_changes/views.py#L223
             * [1]: https://github.com/Kinto/kinto-changes/blob/caa43cca8d0e747756e2cf771f209e3636617663/kinto_changes/views.py#L113-L117
             */
            ("_expected", "0"),
            // Request changes after the last one that we've seen.
            ("_since", &format!("\"{}\"", last_modified)),
            // This order means that if we (for some reason) don't get all
            // pages, the partial sync is still valid.
            ("sort", "-last_modified"),
        ]);

        let mut all_changes = Vec::new();
        let mut next_url = Some(records_url);

        while let Some(url) = next_url.take() {
            tracing::debug!(url = %url.to_string(), "fetching changesets");
            let records_res = self
                .reqwest_client
                .get(url.clone())
                .send()
                .await
                .and_then(Response::error_for_status)
                .context(format!("Fetching records from remote settings: {}", url))
                .map_err(SetupError::Network)?;

            next_url = match records_res.headers().get("Next-Page").map(|header| {
                let s = HeaderValue::to_str(header).context("header as utf8")?;
                Url::parse(s).context("header as url")
            }) {
                Some(Ok(u)) => Some(u),
                Some(Err(error)) => {
                    tracing::warn!(?error, "Invalid Next header from Remote Settings");
                    None
                }
                None => None,
            };

            let ChangesetResponse { changes } = records_res
                .json()
                .await
                .context("Parsing changeset")
                .map_err(SetupError::Format)?;
            all_changes.extend(changes.into_iter());
        }

        tracing::debug!(count = %all_changes.len(), "synced new records");

        for record in all_changes {
            if record.deleted.unwrap_or(false) {
                self.remove_record(record);
            } else {
                self.add_record(record).await.map_err(SetupError::Network)?;
            }
        }

        Ok(())
    }

    /// Iterate over all records in the collection that have a `type` key that
    /// matches the passed value.
    pub fn records_of_type(&self, r#type: String) -> impl Iterator<Item = &Record> {
        RecordsOfTypeIterator {
            records_iter: self.records(),
            r#type,
        }
    }

    /// Remove the record from the local set. `record` should be a tombstone.
    fn remove_record(&mut self, record: Record) {
        debug_assert!(record.deleted.unwrap_or(false));
        if let Some(attachment_meta) = record.attachment {
            self.attachments.remove(&attachment_meta.hash);
        }
        self.records.remove(&record.id);
    }

    /// Add a record to the local set. `record` should not be a tombstone.
    async fn add_record(&mut self, mut record: Record) -> anyhow::Result<()> {
        debug_assert!(!record.deleted.unwrap_or(false));

        // Check for attachment metadata, and if so store it `self.attachments`.
        if let Some(attachment_json) = record.extra.remove("attachment") {
            let hash = attachment_json["hash"]
                .as_str()
                .ok_or_else(|| anyhow!("Invalid attachment hash"))?
                .to_string();

            let attachment = LazyAttachment::build()
                .attachment_base(self.attachment_base_url().await?.clone())
                .attachment_endpoint(
                    attachment_json["location"]
                        .as_str()
                        .ok_or_else(|| anyhow!("Invalid attachment location"))?
                        .to_string(),
                )
                .hash(hash.clone())
                .reqwest_client(self.reqwest_client.clone())
                .finish()?;

            let attachment = Arc::new(attachment);
            self.attachments.insert(hash, attachment.clone());
            record.attachment = Some(attachment);
        }

        self.records.insert(record.id.clone(), record);

        Ok(())
    }

    /// An iterator over the records.
    pub fn records(&self) -> hash_map::Values<String, Record> {
        self.records.values()
    }

    /// Get the attachment base url for the server, either by fetching server
    /// metadata, or by pulling it from our cache.
    async fn attachment_base_url(&mut self) -> anyhow::Result<&Url> {
        if let Some(ref attachment_base_url) = self.attachment_base_url {
            Ok(attachment_base_url)
        } else {
            let server_info =
                RemoteSettingsServerInfo::fetch(&self.server_url, &self.reqwest_client)
                    .await
                    .context("Getting server info")?;
            let s = server_info
                .attachment_base_url()
                .context("Getting attachment base url")?;
            let url = Url::parse(s).context(format!("Attachment url is invalid: {}", s))?;
            self.attachment_base_url = Some(url);
            self.attachment_base_url
                .as_ref()
                .ok_or_else(|| anyhow!("Bug: attachment_base_url not set"))
        }
    }
}

/// A response from the changeset API.
#[derive(Debug, Deserialize, Serialize)]
struct ChangesetResponse {
    /// The records that have changed.
    changes: Vec<Record>,
}

/// A lazy attachment object, representing a value that could be downloaded.
#[derive(Serialize)]
pub struct LazyAttachment {
    /// The URL where the attachment can be downloaded from
    pub location: Url,

    /// The hash of item that should be downloaded.
    pub hash: String,

    /// The cached attachment, unparsed.
    #[serde(skip)]
    downloaded: RwLock<Option<Vec<u8>>>,

    /// Reqwest client to download attachments with
    #[serde(skip)]
    reqwest_client: reqwest::Client,
}

impl Debug for LazyAttachment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LazyAttachment")
            .field("location", &self.location)
            .field("hash", &self.hash)
            .field(
                "downloaded",
                &self.downloaded.try_read().map_or_else(
                    |err| match err {
                        std::sync::TryLockError::Poisoned(_) => "<poisoned>",
                        std::sync::TryLockError::WouldBlock => "<would block>",
                    },
                    |guarded| {
                        if guarded.is_some() {
                            "<cached download>"
                        } else {
                            "<empty>"
                        }
                    },
                ),
            )
            .finish()
    }
}

impl LazyAttachment {
    /// Make a `LazyAttachment` with a builder.
    fn build() -> LazyAttachmentBuilder {
        LazyAttachmentBuilder::default()
    }

    /// Get the attachment associated with this metadata, possibly from a cached copy.
    pub async fn fetch<'de, A: 'de>(&'de self) -> Result<A, SetupError>
    where
        A: DeserializeOwned,
    {
        {
            let downloaded_read_lock = self
                .downloaded
                .read()
                .map_err(|err| SetupError::Io(anyhow!("{}", err)))?;
            if let Some(downloaded) = &*downloaded_read_lock {
                tracing::trace!(hash = ?self.hash, "Reusing existing downloaded attachment");
                let rv: A = serde_json::from_slice(downloaded)
                    .context("Parsing attachment")
                    .map_err(SetupError::Format)?;
                return Ok(rv);
            }
        }

        tracing::trace!(hash = %self.hash, url = %self.location.to_string(), "Downloading attachment");
        let res = self
            .reqwest_client
            .get(self.location.clone())
            .send()
            .await
            .and_then(reqwest::Response::error_for_status)
            .context("downloading attachment request")
            .map_err(SetupError::Network)?;
        let bytes = res
            .bytes()
            .await
            .context("downloading attachment")
            .map_err(SetupError::Network)?
            .to_vec();

        let mut downloaded_write_lock = self
            .downloaded
            .write()
            .map_err(|err| SetupError::Io(anyhow!("{}", err)))?;
        *downloaded_write_lock = Some(bytes);

        serde_json::from_slice(
            downloaded_write_lock
                .as_ref()
                .expect("bug: attachment cache not filled"),
        )
        .context("Parsing attachment")
        .map_err(SetupError::Format)
    }
}

/// Builder for [`LazyAttachment`].
#[derive(Debug, Default)]
#[allow(clippy::missing_docs_in_private_items)]
struct LazyAttachmentBuilder {
    attachment_base: Option<Url>,
    attachment_endpoint: Option<String>,
    hash: Option<String>,
    reqwest_client: Option<reqwest::Client>,
}

impl LazyAttachmentBuilder {
    /// Build the final object, if the previously provided fields are all correct.
    fn finish(self) -> anyhow::Result<LazyAttachment> {
        let attachment_url = self
            .attachment_base
            .ok_or_else(|| anyhow!("attachment base is required"))?
            .join(
                self.attachment_endpoint
                    .as_ref()
                    .ok_or_else(|| anyhow!("attachment endpoint is required"))?,
            )
            .context("Could not build attachment URL")?;

        Ok(LazyAttachment {
            location: attachment_url,
            hash: self.hash.ok_or_else(|| anyhow!("hash is required"))?,
            downloaded: RwLock::new(None),
            reqwest_client: self.reqwest_client.unwrap_or_else(reqwest::Client::new),
        })
    }

    /// Set the attachment base URL. This ill be combined with `attachment_endpoint`.
    fn attachment_base(mut self, attachment_base: Url) -> Self {
        self.attachment_base = Some(attachment_base);
        self
    }

    /// Set the attachment URL's path. This will combined with `attachment_base`.
    fn attachment_endpoint(mut self, attachment_endpoint: String) -> Self {
        self.attachment_endpoint = Some(attachment_endpoint);
        self
    }

    /// Set the expected hash of the contents of this attachment.
    fn hash(mut self, hash: String) -> Self {
        self.hash = Some(hash);
        self
    }

    /// Provide a reqwest client to make download the attachment with.
    fn reqwest_client(mut self, reqwest_client: reqwest::Client) -> Self {
        self.reqwest_client = Some(reqwest_client);
        self
    }
}

/// A record in the Remote Settings collection
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Record {
    /// The Remote Settings ID for this record.
    pub id: String,
    /// The last time this record was modified.
    pub last_modified: u64,
    /// Whether this record is a tombstone
    deleted: Option<bool>,
    /// An attachment for this record, if it exists.
    #[serde(skip_deserializing)]
    attachment: Option<Arc<LazyAttachment>>,

    /// Any extra fields on the record.
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl Record {
    /// Get a reference to the attachment related to this record, if it exists.
    pub fn attachment(&self) -> Option<&LazyAttachment> {
        self.attachment.as_ref().map(Arc::as_ref)
    }
}

/// Iterator for [`RemoteSettingsClient::record_of_type`].
struct RecordsOfTypeIterator<'a, I: Iterator<Item = &'a Record>> {
    /// The records to iterate over
    records_iter: I,
    /// The value to match against for the `type` key.
    r#type: String,
}

impl<'a, I: Iterator<Item = &'a Record>> Iterator for RecordsOfTypeIterator<'a, I> {
    type Item = &'a Record;

    fn next(&mut self) -> Option<Self::Item> {
        let t = &self.r#type;
        self.records_iter.find(|r| {
            r.extra
                .get("type")
                .and_then(Value::as_str)
                .map_or(false, |v| v == t)
        })
    }
}

/// Remote Settings server info
#[derive(Debug, Deserialize, Serialize)]
struct RemoteSettingsServerInfo {
    /// The capabilities the server supports.
    capabilities: RemoteSettingsCapabilities,
}

impl RemoteSettingsServerInfo {
    /// Fetch a copy of the server info from the default Remote Settings server with the provided client.
    async fn fetch(server_url: &Url, client: &reqwest::Client) -> anyhow::Result<Self> {
        let res = client
            .get(
                server_url
                    .join("/v1/")
                    .context("building server info URL")?,
            )
            .send()
            .await
            .and_then(Response::error_for_status)
            .context("Fetching RemoteSettings server info")?;
        let server_info: Self = res
            .json()
            .await
            .context("Parsing RemoteSettings server info")?;
        Ok(server_info)
    }

    /// Get the attachment base URL. Returns an error if the server does not support attachments.
    fn attachment_base_url(&self) -> Result<&str, SetupError> {
        Ok(&self
            .capabilities
            .attachments
            .as_ref()
            .ok_or_else(|| {
                SetupError::InvalidConfiguration(anyhow!(
                    "Remote settings does not support required extension: attachments"
                ))
            })?
            .base_url)
    }
}

/// Remote Settings server capabilities
#[derive(Debug, Deserialize, Serialize)]
struct RemoteSettingsCapabilities {
    /// The attachments capability. `None` if the server does not support attachments.
    attachments: Option<RemoteSettingsAttachmentsCapability>,
}

/// Remote Settings attachments capability
#[derive(Debug, Deserialize, Serialize)]
struct RemoteSettingsAttachmentsCapability {
    /// The URL that attachments' `location` field is relative to
    base_url: String,
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, str::FromStr, sync::Arc};

    use crate::remote_settings::client::{LazyAttachment, Record};

    use super::{
        ChangesetResponse, RemoteSettingsAttachmentsCapability, RemoteSettingsCapabilities,
        RemoteSettingsClient, RemoteSettingsServerInfo,
    };
    use merino_settings::providers::RemoteSettingsConfig;
    use reqwest::Url;

    #[actix_rt::test]
    async fn test_sync_makes_expected_call() -> anyhow::Result<()> {
        let config = RemoteSettingsConfig::default();

        let mock_server = httpmock::MockServer::start();

        let records_mock = mock_server
            .mock_async(|when, then| {
                when.path(format!(
                    "/v1/buckets/{}/collections/{}/changeset",
                    config.bucket, config.collection
                ));
                then.json_body_obj(&ChangesetResponse { changes: vec![] });
            })
            .await;

        let mut client =
            RemoteSettingsClient::new(&mock_server.base_url(), config.bucket, config.collection)?;
        client.sync().await?;

        records_mock.assert();

        Ok(())
    }

    #[actix_rt::test]
    async fn test_sync_two_pages() -> anyhow::Result<()> {
        let config = RemoteSettingsConfig::default();

        let mock_server = httpmock::MockServer::start();

        let page_1_mock = mock_server.mock(|when, then| {
            when.path(format!(
                "/v1/buckets/{}/collections/{}/changeset",
                config.bucket, config.collection
            ));
            then.header("Next-Page", &mock_server.url("/page-2"))
                .json_body_obj(&ChangesetResponse { changes: vec![] });
        });

        let page_2_mock = mock_server.mock(|when, then| {
            when.path("/page-2");
            then.json_body_obj(&ChangesetResponse { changes: vec![] });
        });

        let mut client =
            RemoteSettingsClient::new(&mock_server.base_url(), config.bucket, config.collection)?;
        client.sync().await?;

        page_1_mock.assert();
        page_2_mock.assert();

        Ok(())
    }

    #[actix_rt::test]
    async fn test_attachments_work() -> anyhow::Result<()> {
        let config = RemoteSettingsConfig::default();
        let mock_server = httpmock::MockServer::start();
        let attachment_base_url = Url::from_str(&mock_server.base_url())
            .unwrap()
            .join("attachments/")?;

        let server_info_mock = mock_server
            .mock_async(|when, then| {
                when.path("/v1/");
                then.json_body_obj(&RemoteSettingsServerInfo {
                    capabilities: RemoteSettingsCapabilities {
                        attachments: Some(RemoteSettingsAttachmentsCapability {
                            base_url: attachment_base_url.to_string(),
                        }),
                    },
                });
            })
            .await;

        let attachment = Arc::new(
            LazyAttachment::build()
                .attachment_base(attachment_base_url)
                .attachment_endpoint("apple.txt".to_string())
                .hash("fake hash".to_string())
                .finish()?,
        );
        let records_mock = mock_server
            .mock_async(|when, then| {
                when.path(format!(
                    "/v1/buckets/{}/collections/{}/changeset",
                    config.bucket, config.collection
                ));
                then.json_body_obj(&ChangesetResponse {
                    changes: vec![Record {
                        attachment: Some(attachment),
                        deleted: None,
                        extra: HashMap::default(),
                        id: "apple-id".to_string(),
                        last_modified: 0,
                    }],
                });
            })
            .await;

        let attachment_mock = mock_server
            .mock_async(|when, then| {
                when.path("/attachments/apple.txt");
                then.body("1");
            })
            .await;

        let mut client =
            RemoteSettingsClient::new(&mock_server.base_url(), config.bucket, config.collection)?;
        client.sync().await?;

        // Download every attachment and make sure it is what we expect
        let mut attachment_values: Vec<u32> = vec![];
        for record in client.records() {
            attachment_values.push(record.attachment().unwrap().fetch().await?);
        }

        server_info_mock.assert_hits(1);
        attachment_mock.assert_hits(1);
        records_mock.assert_hits(1);
        assert_eq!(attachment_values, vec![1]);

        Ok(())
    }
}
