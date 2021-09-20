//! AdM integration that uses the remote-settings provided data.

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use futures::StreamExt;
use http::{HeaderValue, Uri};
use lazy_static::lazy_static;
use merino_settings::{providers::RemoteSettingsConfig, Settings};
use merino_suggest::{
    Proportion, SetupError, SuggestError, Suggestion, SuggestionProvider, SuggestionRequest,
    SuggestionResponse,
};
use reqwest::Response;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::{collections::HashMap, convert::TryFrom, sync::Arc};

lazy_static! {
    static ref NON_SPONSORED_IAB_CATEGORIES: Vec<&'static str> = vec!["5 - Education"];
}

/// Make suggestions based on data in Remote Settings
#[derive(Default, Debug)]
pub struct RemoteSettingsSuggester {
    /// A map from keywords to suggestions that can be provided.
    suggestions: HashMap<String, Arc<Suggestion>>,
}

impl RemoteSettingsSuggester {
    /// Make and sync a new suggester.
    pub async fn new_boxed(
        settings: &Settings,
        config: &RemoteSettingsConfig,
    ) -> Result<Box<Self>, SetupError> {
        let mut provider = Self {
            suggestions: HashMap::new(),
        };
        provider.sync(settings, config).await?;
        Ok(Box::new(provider))
    }

    /// Download suggestions from Remote Settings
    ///
    /// This must be called at least once before any suggestions will be provided
    #[tracing::instrument(skip(self, settings))]
    pub async fn sync(
        &mut self,
        settings: &Settings,
        config: &RemoteSettingsConfig,
    ) -> Result<(), SetupError> {
        tracing::info!(
            r#type = "adm.remote-settings.sync-start",
            "Syncing quicksuggest records from Remote Settings"
        );
        let reqwest_client = reqwest::Client::new();

        // Get the base URL to download attachments from.
        let remote_settings_server_info =
            RemoteSettingsServerInfo::fetch(settings, &reqwest_client).await?;
        let attachment_base_url = remote_settings_server_info.attachment_base_url()?;

        // Get records from Remote Settings.
        let records = {
            let records_url = format!(
                "{}/v1/buckets/{}/collections/{}/records",
                settings.remote_settings.server, config.bucket, config.collection
            );

            let mut all_records = Vec::new();
            let mut next_url = Some(records_url);
            while let Some(url) = next_url {
                let records_res = reqwest_client
                    .get(&url)
                    .send()
                    .await
                    .and_then(Response::error_for_status)
                    .context(format!("Fetching records from remote settings: {}", url))
                    .map_err(SetupError::Network)?;

                next_url = match records_res
                    .headers()
                    .get("Next-Page")
                    .map(HeaderValue::to_str)
                {
                    Some(Ok(u)) => Some(u.to_string()),
                    Some(Err(error)) => {
                        tracing::warn!(?error, "Invalid Next header from Remote Settings");
                        None
                    }
                    None => None,
                };

                let RecordsResponse { data: records } = records_res
                    .json()
                    .await
                    .context("Parsing suggestion records")
                    .map_err(SetupError::Format)?;
                all_records.extend(records.into_iter());
            }
            all_records
        };

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
                    score: Proportion::from(0.2),
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
impl SuggestionProvider for RemoteSettingsSuggester {
    fn name(&self) -> String {
        "AdmRemoteSettings".into()
    }

    async fn suggest(
        &self,
        request: SuggestionRequest,
    ) -> Result<SuggestionResponse, SuggestError> {
        let suggestions = if request.accepts_english {
            match self.suggestions.get(&request.query) {
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
#[derive(Debug, Deserialize, Serialize)]
struct RemoteSettingsServerInfo {
    /// The capabilities the server supports.
    capabilities: RemoteSettingsCapabilities,
}

impl RemoteSettingsServerInfo {
    /// Fetch a copy of the server info from the default Remote Settings server with the provided client.
    async fn fetch(settings: &Settings, client: &reqwest::Client) -> Result<Self, SetupError> {
        let res = client
            .get(format!("{}/v1/", settings.remote_settings.server))
            .send()
            .await
            .and_then(Response::error_for_status)
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

/// The result of the /records endpoint on a collection.
#[derive(Debug, Deserialize, Serialize)]
struct RecordsResponse {
    /// The records returned.
    data: Vec<SuggestRecord>,
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

/// A record stored in the Remote Settings quicksuggest collection.
///
/// This is a non-exhaustive description of the records in the collection, only
/// including fields needed to retrieve suggestions.
#[derive(Debug, Deserialize, Serialize)]
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
#[derive(Debug, Deserialize, Serialize)]
struct AttachmentMeta {
    /// The location the attachment can be downloaded from, relative to the
    /// attachment base_url specified in the server capabilities.
    location: String,
}

/// A suggestion record from AdM
#[serde_as]
#[derive(Debug, Deserialize, Serialize)]
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
                score: Proportion::zero(),
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
                score: Proportion::zero(),
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

    #[actix_rt::test]
    async fn test_sync_makes_expected_call() -> anyhow::Result<()> {
        let config = RemoteSettingsConfig::default();

        let mock_server = httpmock::MockServer::start();
        let settings = {
            let mut settings = Settings::load_for_tests();
            settings.remote_settings.server = format!("http://{}", mock_server.address());
            settings
        };

        let server_info_mock = mock_server.mock(|when, then| {
            when.path("/v1/");
            then.json_body_obj(&RemoteSettingsServerInfo {
                capabilities: RemoteSettingsCapabilities {
                    attachments: Some(RemoteSettingsAttachmentsCapability {
                        base_url: settings.remote_settings.server.clone(),
                    }),
                },
            });
        });

        let records_mock = mock_server.mock(|when, then| {
            when.path(format!(
                "/v1/buckets/{}/collections/{}/records",
                config.bucket, config.collection
            ));
            then.json_body_obj(&RecordsResponse { data: vec![] });
        });

        let mut provider = RemoteSettingsSuggester::default();
        provider.sync(&settings, &config).await?;

        server_info_mock.assert();
        records_mock.assert();

        Ok(())
    }

    #[actix_rt::test]
    async fn test_sync_two_pages() -> anyhow::Result<()> {
        let config = RemoteSettingsConfig::default();

        let mock_server = httpmock::MockServer::start();
        let settings = {
            let mut settings = Settings::load_for_tests();
            settings.remote_settings.server = format!("http://{}", mock_server.address());
            settings
        };

        let server_info_mock = mock_server.mock(|when, then| {
            when.path("/v1/");
            then.status(200).json_body_obj(&RemoteSettingsServerInfo {
                capabilities: RemoteSettingsCapabilities {
                    attachments: Some(RemoteSettingsAttachmentsCapability {
                        base_url: settings.remote_settings.server.clone(),
                    }),
                },
            });
        });

        let page_1_mock = mock_server.mock(|when, then| {
            when.path(format!(
                "/v1/buckets/{}/collections/{}/records",
                config.bucket, config.collection
            ));
            then.header("Next-Page", &mock_server.url("/page-2"))
                .json_body_obj(&RecordsResponse { data: vec![] });
        });

        let page_2_mock = mock_server.mock(|when, then| {
            when.path("/page-2");
            then.json_body_obj(&RecordsResponse { data: vec![] });
        });

        let mut provider = RemoteSettingsSuggester::default();
        provider.sync(&settings, &config).await?;

        server_info_mock.assert();
        page_1_mock.assert();
        page_2_mock.assert();

        Ok(())
    }
}
