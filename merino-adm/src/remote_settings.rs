//! AdM integration that uses the remote-settings provided data.

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use futures::StreamExt;
use http::Uri;
use lazy_static::lazy_static;
use merino_settings::Settings;
use merino_suggest::{
    SetupError, SuggestError, Suggestion, SuggestionProvider, SuggestionRequest, SuggestionResponse,
};
use remote_settings_client::client::FileStorage;
use serde::Deserialize;
use serde_json::Value;
use serde_with::{serde_as, DisplayFromStr};
use std::{borrow::Cow, collections::HashMap, convert::TryFrom, sync::Arc};
use tokio::sync::OnceCell;

lazy_static! {
    static ref NON_SPONSORED_IAB_CATEGORIES: Vec<&'static str> = vec!["5 - Education"];
}

/// Make suggestions based on data in Remote Settings
#[derive(Default, Debug)]
pub struct RemoteSettingsSuggester {
    /// A map from keywords to suggestions that can be provided.
    suggestions: HashMap<String, Arc<Suggestion>>,
}

/// A lazy version of the server settings for the default Remote Settings server.
/// Should be initialized with `RemoteSettingsServerInfo::fetch`.
static REMOTE_SETTINGS_SERVER_INFO: OnceCell<RemoteSettingsServerInfo> = OnceCell::const_new();

impl RemoteSettingsSuggester {
    /// Make and sync a new suggester.
    pub async fn new_boxed(settings: &Settings) -> Result<Box<Self>, SetupError> {
        let mut provider = Self {
            suggestions: HashMap::new(),
        };
        provider.sync(settings).await?;
        Ok(Box::new(provider))
    }

    /// Download suggestions from Remote Settings
    ///
    /// This must be called at least once before any suggestions will be provided
    #[tracing::instrument(skip(self, settings))]
    pub async fn sync(&mut self, settings: &Settings) -> Result<(), SetupError> {
        tracing::info!(
            r#type = "adm.remote-settings.sync-start",
            "Syncing quicksuggest records from Remote Settings"
        );
        let provider_settings = &settings.providers.adm_rs;
        let reqwest_client = reqwest::Client::new();

        // Set up and sync a Remote Settings client for the quicksuggest collection.
        std::fs::create_dir_all(&provider_settings.storage_path)
            .context("Creating RemoteSettings file cache")
            .map_err(SetupError::Io)?;
        let mut rs_client = {
            let mut rs_client_builder = remote_settings_client::Client::builder()
                .collection_name(&provider_settings.collection)
                .storage(Box::new(FileStorage {
                    folder: provider_settings.storage_path.clone(),
                    ..Default::default()
                }));
            if let Some(server) = &provider_settings.server {
                rs_client_builder = rs_client_builder.server_url(server);
            }
            rs_client_builder
                .build()
                .context("Creating RemoteSettings client")
                .map_err(SetupError::InvalidConfiguration)?
        };

        // `.sync()` blocks while doing IO
        rs_client
            .sync(None)
            .context("Syncing suggestions from remote settings")
            .map_err(SetupError::Network)?;

        // Get the base URL to download attachments from.
        let attachment_base_url = &REMOTE_SETTINGS_SERVER_INFO
            .get_or_try_init(|| RemoteSettingsServerInfo::fetch(&reqwest_client))
            .await?
            .attachment_base_url()?;

        // Get records from Remote Settings, and convert them into a schema instead of using JSON `Value`s.
        let records: Vec<SuggestRecord> = rs_client
            // `.get()` blocks while doing IO
            .get()
            .context("Fetching records from remote settings")
            .map_err(SetupError::Network)?
            .into_iter()
            .filter(|r| !r.deleted())
            .map(|r| {
                let value = Value::Object(r.as_object().clone());
                <SuggestRecord as Deserialize>::deserialize(value)
            })
            .collect::<Result<_, <Value as serde::Deserializer>::Error>>()
            .context("Parsing suggestions records")
            .map_err(SetupError::Format)?;

        // Sort records by type
        let mut records_by_type: HashMap<&str, Vec<&SuggestRecord>> =
            records.iter().fold(HashMap::new(), |mut acc, record| {
                acc.entry(&record.record_type)
                    .or_insert_with(Vec::new)
                    .push(record);
                acc
            });

        // Build a map of icon IDs to URLs.
        let icon_urls: HashMap<String, String> = records_by_type
            .entry("icon")
            .or_default()
            .iter()
            .flat_map(|record| {
                record.attachment.as_ref().map(|attachment| {
                    let url = format!("{}{}", attachment_base_url, attachment.location);
                    (record.id.clone(), url)
                })
            })
            .collect();

        // The suggestion options are stored in attachments instead of directly in the RS records.
        let suggestion_attachment_metas: Vec<_> = records_by_type
            .entry("data")
            .or_default()
            .iter()
            .flat_map(|r| r.attachment.as_ref())
            .collect();

        // Download all the attachments concurrently
        let mut suggestion_attachments = futures::stream::FuturesUnordered::new();
        for attachment_meta in suggestion_attachment_metas {
            let reqwest_client = &reqwest_client;
            let url = format!("{}{}", attachment_base_url, attachment_meta.location);
            suggestion_attachments.push(async move {
                let res = reqwest_client
                    .get(&url)
                    .send()
                    .await
                    .and_then(|res| res.error_for_status())
                    .context("Fetching suggestion attachments (connection)")
                    .map_err(SetupError::Network)?;
                let rv: Vec<AdmSuggestion> = res
                    .json()
                    .await
                    .context("Parsing suggestions")
                    .map_err(SetupError::Format)?;
                Result::<Vec<AdmSuggestion>, SetupError>::Ok(rv)
            });
        }

        // Convert the collection of adM suggestion attachments into a lookup
        // table of keyword -> merino suggestion.
        let mut suggestions = HashMap::new();
        while let Some(attachment) = suggestion_attachments.next().await {
            for adm_suggestion in attachment? {
                if adm_suggestion.keywords.is_empty() {
                    continue;
                }

                let icon_key = format!("icon-{}", adm_suggestion.icon);
                let icon_url = match icon_urls.get(&icon_key) {
                    Some(s) => match Uri::try_from(s) {
                        Ok(url) => url,
                        Err(error) => {
                            tracing::warn!(suggestion_id = %adm_suggestion.id, %error, url = %s, "ADM suggestion has invalid icon URL");
                            continue;
                        }
                    },
                    None => {
                        tracing::warn!(suggestion_id = %adm_suggestion.id, "ADM suggestion has no icon");
                        continue;
                    }
                };

                let full_keyword = adm_suggestion
                    .keywords
                    .iter()
                    .max_by_key(|kw| kw.len())
                    .expect("No keywords?")
                    .clone();

                let merino_suggestion = Arc::new(Suggestion {
                    id: adm_suggestion.id,
                    title: adm_suggestion.title.clone(),
                    url: adm_suggestion.url.clone(),
                    impression_url: adm_suggestion.impression_url,
                    click_url: adm_suggestion.click_url,
                    full_keyword,
                    provider: adm_suggestion.advertiser,
                    is_sponsored: !NON_SPONSORED_IAB_CATEGORIES
                        .contains(&adm_suggestion.iab_category.as_str()),
                    icon: icon_url,
                });
                for keyword in adm_suggestion.keywords {
                    suggestions.insert(keyword, merino_suggestion.clone());
                }
            }
        }

        if suggestions.is_empty() {
            tracing::warn!(
                r#type = "adm.remote-settings.empty",
                "No suggestion records found on Remote Settings"
            );
        }

        self.suggestions = suggestions;
        tracing::info!(
            r#type = "adm.remote-settings.sync-done",
            "Completed syncing quicksuggest records from Remote Settings"
        );

        Ok(())
    }
}

#[async_trait]
impl<'a> SuggestionProvider<'a> for RemoteSettingsSuggester {
    fn name(&self) -> std::borrow::Cow<'a, str> {
        Cow::from("AdmRemoteSettings")
    }

    async fn suggest(
        &self,
        request: SuggestionRequest<'a>,
    ) -> Result<SuggestionResponse, SuggestError> {
        let suggestions = if request.accepts_english {
            match self.suggestions.get(request.query.as_ref()) {
                Some(suggestion) => vec![suggestion.as_ref().clone()],
                _ => vec![],
            }
        } else {
            vec![]
        };

        Ok(SuggestionResponse::new(suggestions))
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
    async fn fetch(client: &reqwest::Client) -> Result<Self, SetupError> {
        let res = client
            .get(remote_settings_client::DEFAULT_SERVER_URL)
            .send()
            .await
            .and_then(|res| res.error_for_status())
            .context("Fetching RemoteSettings server info")
            .map_err(SetupError::Network)?;
        let server_info: Self = res
            .json()
            .await
            .context("Parsing RemoteSettings server info")
            .map_err(SetupError::Format)?;
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
    /// Record ID
    id: String,

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
#[serde_as]
#[derive(Debug, Deserialize)]
#[allow(clippy::missing_docs_in_private_items)]
struct AdmSuggestion {
    id: u32,
    #[serde_as(as = "DisplayFromStr")]
    url: Uri,
    #[serde_as(as = "DisplayFromStr")]
    click_url: Uri,
    #[serde_as(as = "DisplayFromStr")]
    impression_url: Uri,
    iab_category: String,
    #[serde_as(as = "DisplayFromStr")]
    icon: u64,
    advertiser: String,
    title: String,
    keywords: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use fake::{Fake, Faker};
    use merino_suggest::{Suggestion, SuggestionProvider};

    #[actix_rt::test]
    async fn english_is_supported_example() -> anyhow::Result<()> {
        let mut suggestions = HashMap::new();
        suggestions.insert(
            "sheep".to_string(),
            Arc::new(Suggestion {
                title: "Wikipedia - Sheep".to_string(),
                url: Uri::from_static("https://en.wikipedia.org/wiki/Sheep"),
                id: 1,
                full_keyword: "sheep".to_string(),
                impression_url: Uri::from_static("https://127.0.0.1"),
                click_url: Uri::from_static("https://127.0.0.1"),
                provider: "test".to_string(),
                is_sponsored: false,
                icon: Uri::from_static("https://en.wikipedia.org/favicon.ico"),
            }),
        );
        let rs_suggester = RemoteSettingsSuggester { suggestions };

        let request = SuggestionRequest {
            query: "sheep".into(),
            accepts_english: true,
            ..Faker.fake()
        };

        assert_eq!(
            rs_suggester
                .suggest(request)
                .await?
                .suggestions
                .iter()
                .map(|s| &s.title)
                .collect::<Vec<_>>(),
            vec!["Wikipedia - Sheep"]
        );

        Ok(())
    }

    #[actix_rt::test]
    async fn english_is_unsupported_example() -> anyhow::Result<()> {
        let mut suggestions = HashMap::new();
        suggestions.insert(
            "sheep".to_string(),
            Arc::new(Suggestion {
                title: "Wikipedia - Sheep".to_string(),
                url: Uri::from_static("https://en.wikipedia.org/wiki/Sheep"),
                id: 1,
                full_keyword: "sheep".to_string(),
                impression_url: Uri::from_static("https://127.0.0.1"),
                click_url: Uri::from_static("https://127.0.0.1"),
                provider: "test".to_string(),
                is_sponsored: false,
                icon: Uri::from_static("https://en.wikipedia.org/favicon.ico"),
            }),
        );
        let rs_suggester = RemoteSettingsSuggester { suggestions };

        let request = SuggestionRequest {
            query: "sheep".into(),
            accepts_english: false,
            ..Faker.fake()
        };

        assert!(rs_suggester.suggest(request).await?.suggestions.is_empty());

        Ok(())
    }
}
