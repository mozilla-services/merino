//! Helpers to make working with the ES API easier. Customized specifically for
//! our use case and Wikipedia content.

use anyhow::{bail, Context, Result};
use elasticsearch::{
    http::transport::{CloudConnectionPool, SingleNodeConnectionPool, TransportBuilder},
    indices::{IndicesCreateParts, IndicesExistsParts},
    Elasticsearch, IndexParts,
};
use serde::Serialize;
use serde_json::json;

use crate::{WikipediaDocument, WikipediaNamespace};

/// A wrapper around an Elasticsearch client to make common operations easier
/// and more consistent.
pub struct ElasticHelper {
    /// The ES client this helper users.
    pub client: Elasticsearch,
    /// The search index this helper targets.
    pub index_name: String,
}

impl ElasticHelper {
    /// Create a new ElasticHelper that connects to the server specified in
    /// `settings` and uses the index specified in [`index_name`].
    ///
    /// # Errors
    /// If the settings to connect to Elasticsearch are not valid, the creation
    /// process may fail.
    pub fn new<S: Into<String>>(
        es_settings: &merino_settings::ElasticsearchSettings,
        index_name: S,
    ) -> Result<Self> {
        let transport_builder = match &es_settings.connection {
            merino_settings::ElasticsearchConnection::Single { url } => {
                TransportBuilder::new(SingleNodeConnectionPool::new(
                    elasticsearch::http::Url::parse(&url.to_string())
                        .context("Could not parse Elasticsearch URL")?,
                ))
            }
            merino_settings::ElasticsearchConnection::Cloud { cloud_id } => TransportBuilder::new(
                CloudConnectionPool::new(cloud_id)
                    .context("Could not create Elasticsearch cloud connection")?,
            ),
            merino_settings::ElasticsearchConnection::None => {
                bail!("Elasticsearch is not configured")
            }
        };

        let transport = transport_builder
            .build()
            .context("misconfigured elasticsearch")?;

        let client = Elasticsearch::new(transport);

        Ok(Self {
            client,
            index_name: index_name.into(),
        })
    }

    /// Check that the helper's index exists.
    /// # Errors
    /// If there is an HTTP error communication with ES.
    pub async fn index_exists(&self) -> Result<bool> {
        let exists_res = self
            .client
            .indices()
            .exists(IndicesExistsParts::Index(&[&self.index_name]))
            .send()
            .await
            .context(format!("index_exists({}) request", self.index_name))?;
        match exists_res.status_code().as_u16() {
            200 => Ok(true),
            404 => Ok(false),
            code => {
                let exists_output = exists_res
                    .text()
                    .await
                    .context("Fetching text of exists command")?;
                Err(anyhow::anyhow!(
                    "Unexpected status code from index_exists({}) {code}. Error: {exists_output}",
                    self.index_name
                ))
            }
        }
    }

    /// Creates the helper's index, assuming it doesn't already exist.
    /// # Errors
    /// If the index already exists, or if there is an HTTP error communication with ES.
    pub async fn index_create(&self) -> Result<()> {
        let create_res = self
            .client
            .indices()
            .create(IndicesCreateParts::Index(&self.index_name))
            .body(json!({
                "mappings": {
                    "_source": {
                        "excludes": ["page_text"],
                    },
                    "properties": {
                        "title": {"type": "text", "store": true, "analyzer": "english"},
                        "page_text": {"type": "search_as_you_type", "store": false, "analyzer": "english"},
                        "url": {"type": "keyword", "index": false, "store": true},
                        "namespace_id": {"type": "keyword", "index": false},
                        "page_id": {"type": "keyword", "index": false},
                    }
                }
            }))
            .send()
            .await
            .context(format!("index_create({}) request", self.index_name))?;

        let status = create_res.status_code();
        if status.is_success() {
            Ok(())
        } else {
            let exists_output = create_res
                .text()
                .await
                .context("Fetching text of exists command")?;
            Err(anyhow::anyhow!(
                "Unexpected status code from index_create({}) {}. Error: {exists_output}",
                self.index_name,
                status.as_u16(),
            ))
        }
    }

    /// Ensure that the helper's index exists, creating it if needed.
    /// # Errors
    /// If there is an HTTP error communication with ES.
    pub async fn index_ensure_exists(&self) -> Result<()> {
        if !self.index_exists().await? {
            self.index_create().await?;
        }
        Ok(())
    }

    /// Add a document to the helper's index. The ID of the document will be set
    /// based on the [`HelperIndexable::doc_id`] method.
    ///
    /// # Errors
    /// If there is an HTTP error communication with ES, or if there is a
    /// problem serializing the document.
    pub async fn doc_add<T>(&self, doc: T) -> Result<()>
    where
        T: HelperIndexable,
    {
        let doc_id = doc.doc_id();
        self.client
            .index(IndexParts::IndexId(&self.index_name, &doc_id))
            .body(doc)
            .send()
            .await
            .context("indexing doc request")?;
        Ok(())
    }
}

/// An object that can be indexed using [`ElasticHelper`].  pub trait HelperIndexable: Serialize {
pub trait HelperIndexable: Serialize {
    /// The ID that this document should be indexed under. Two documents with
    /// the same ID are considered to be semantically equivalent. If a document
    /// with the given ID is already present in the search index, it will be
    /// overridden if another document with the same ID is added.
    fn doc_id(&self) -> String;
}

impl HelperIndexable for WikipediaDocument {
    fn doc_id(&self) -> String {
        format!(
            "wikidoc/{}/{}",
            <i32 as From<WikipediaNamespace>>::from(self.namespace),
            self.page_id
        )
    }
}
