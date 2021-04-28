//! AdM integration that uses the remote-settings provided data.

use anyhow::{anyhow, Result};
use futures::stream::{self, StreamExt};
use merino_suggest::{Suggester, Suggestion};
use radix_trie::Trie;
use remote_settings_client::client::FileStorage;
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, rc::Rc};
use tokio::sync::OnceCell;

/// Make suggestions based on data in Remote Settings
#[derive(Debug, Default)]
pub struct RemoteSettingsSuggester {
    /// A map from keywords to suggestions that can be provided.
    suggestions: Trie<String, Rc<Suggestion>>,
}

/// A lazy version of the server settings for the default Remote Settings server.
/// Should be initialized with `RemoteSettingsServerInfo::fetch`.
static REMOTE_SETTINGS_SERVER_INFO: OnceCell<RemoteSettingsServerInfo> = OnceCell::const_new();

impl RemoteSettingsSuggester {
    /// Download suggestions from Remote Settings
    ///
    /// This must be called at least once before any suggestions will be provided
    pub async fn sync(&mut self) -> Result<()> {
        let reqwest_client = reqwest::Client::new();

        // Set up and sync a Remote Settings client for the quicksuggest collection.
        std::fs::create_dir_all("./rs-cache")?;
        let mut rs_client = remote_settings_client::Client::builder()
            .collection_name("quicksuggest")
            .storage(Box::new(FileStorage {
                folder: "./rs-cache".into(),
                ..Default::default()
            }))
            .build()
            .map_err(|s| anyhow!("{}", s))?;
        // `.sync()` blocks while doing IO
        rs_client.sync(None)?;

        // Get records from Remote Settings, and convert them into a schema instead of using JSON `Value`s.
        let records: Vec<SuggestRecord> = rs_client
            // `.get()` blocks while doing IO
            .get()?
            .into_iter()
            .filter(|r| !r.deleted())
            .map(|r| {
                let value = Value::Object(r.as_object().clone());
                <SuggestRecord as Deserialize>::deserialize(value)
            })
            .collect::<Result<_, <Value as serde::Deserializer>::Error>>()?;

        // Sort records by type
        let records_by_type: HashMap<&str, Vec<&SuggestRecord>> =
            records.iter().fold(HashMap::new(), |mut acc, record| {
                acc.entry(&record.record_type)
                    .or_insert_with(Vec::new)
                    .push(record);
                acc
            });

        // The suggestion options are stored in attachments instead of directly in the RS records.
        let suggestion_attachment_metas = records_by_type
            .get("data")
            .ok_or_else(|| anyhow!("No data records found"))?
            .iter()
            .flat_map(|r| r.attachment.as_ref());

        // Get the base URL to download attachments from.
        let attachment_base_url = &REMOTE_SETTINGS_SERVER_INFO
            .get_or_try_init(|| RemoteSettingsServerInfo::fetch(&reqwest_client))
            .await?
            .attachment_base_url()?;

        // Download all the attachments, concurrently.
        let suggestion_attachments = stream::iter(suggestion_attachment_metas)
            .map(|attachment_meta| {
                let reqwest_client = &reqwest_client;
                let url = format!("{}{}", attachment_base_url, attachment_meta.location);
                async move {
                    let resp = reqwest_client.get(&url).send().await?.error_for_status()?;
                    let rv: Vec<AdmSuggestion> = resp.json().await?;
                    Result::<Vec<AdmSuggestion>>::Ok(rv)
                }
            })
            .buffer_unordered(5)
            .collect::<Vec<_>>()
            .await;

        // Convert the collection of adM suggestion attachments into a lookup
        // table of keyword -> merino suggestion.
        let mut suggestions = Trie::new();
        for attachment in suggestion_attachments {
            for adm_suggestion in attachment? {
                let merino_suggestion = Rc::new(Suggestion {
                    title: adm_suggestion.title.clone(),
                    url: adm_suggestion.url.clone(),
                });
                for keyword in adm_suggestion.keywords {
                    suggestions.insert(keyword, merino_suggestion.clone());
                }
            }
        }
        self.suggestions = suggestions;

        Ok(())
    }
}

impl Suggester for RemoteSettingsSuggester {
    fn suggest(&self, query: &str) -> Vec<Suggestion> {
        match self.suggestions.get(query) {
            Some(suggestion) => vec![suggestion.as_ref().clone()],
            _ => vec![],
        }
    }
}

/// Remote Settings server info
#[derive(Debug, Deserialize)]
struct RemoteSettingsServerInfo {
    /// The capabilities the server supports.
    capabilities: RemoteSettingsCapabilities,
}

impl RemoteSettingsServerInfo {
    /// Fetch a copy of the server info from the default Remote Settings server with the provided client.
    async fn fetch(client: &reqwest::Client) -> Result<Self> {
        let res = client
            .get(remote_settings_client::DEFAULT_SERVER_URL)
            .send()
            .await?
            .error_for_status()?;
        let server_info: Self = res.json().await?;
        Ok(server_info)
    }

    /// Get the attachment base URL. Returns an error if the server does not support attachments.
    fn attachment_base_url(&self) -> Result<&str> {
        Ok(&self
            .capabilities
            .attachments
            .as_ref()
            .ok_or_else(|| anyhow!("Server does not support attachments"))?
            .base_url)
    }
}

/// Remote Settings server capabilities
#[derive(Debug, Deserialize)]
struct RemoteSettingsCapabilities {
    /// The attachments capability. `None` if the server does not support attachments.
    attachments: Option<RemoteSettingsAttachmentsCapability>,
}

/// Remote Settings attachments capability
#[derive(Debug, Deserialize)]
struct RemoteSettingsAttachmentsCapability {
    /// The URL that attachments' `location` field is relative to
    base_url: String,
}

/// A record stored in the Remote Settings quicksuggest collection.
///
/// This is a non-exhaustive description of the records in the collection, only
/// including fields needed to retrieve suggestions.
#[derive(Deserialize)]
struct SuggestRecord {
    /// Attachment information, if any.
    attachment: Option<AttachmentMeta>,

    /// The type of the record. Expected to be "data" or "icon".
    #[serde(rename = "type")]
    record_type: String,
}

/// The metadata of an attachment that might be associated with a Remote Settings record.
///
/// This is a non-exhaustive description of the records in the collection, only
/// including fields needed to retrieve suggestions.
#[derive(Deserialize)]
struct AttachmentMeta {
    /// The location the attachment can be downloaded from, relative to the
    /// attachment base_url specified in the server capabilities.
    location: String,
}

/// A suggestion record from AdM
#[derive(Debug, Deserialize)]
#[allow(clippy::missing_docs_in_private_items)]
struct AdmSuggestion {
    id: u32,
    url: String,
    click_url: String,
    impression_url: String,
    iab_category: String,
    icon: String,
    advertiser: String,
    title: String,
    keywords: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use merino_suggest::{Suggester, Suggestion};

    #[test]
    fn it_works() {
        let mut suggestions = Trie::new();
        suggestions.insert(
            "sheep".to_string(),
            Rc::new(Suggestion {
                title: "Wikipedia - Sheep".to_string(),
                url: "https://en.wikipedia.org/wiki/Sheep".to_string(),
            }),
        );
        let rs_suggester = RemoteSettingsSuggester { suggestions };

        assert_eq!(
            rs_suggester.suggest("sheep"),
            vec![Suggestion {
                title: "Wikipedia - Sheep".to_string(),
                url: "https://en.wikipedia.org/wiki/Sheep".to_string(),
            }]
        );
    }
}
