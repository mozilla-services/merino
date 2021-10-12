//! AdM integration that uses the remote-settings provided data.

mod client;

use anyhow::anyhow;
use async_trait::async_trait;
use futures::StreamExt;
use http::Uri;
use lazy_static::lazy_static;
use merino_settings::{providers::RemoteSettingsConfig, Settings};
use merino_suggest::{
    Proportion, SetupError, SuggestError, Suggestion, SuggestionProvider, SuggestionRequest,
    SuggestionResponse,
};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::{collections::HashMap, sync::Arc};

use crate::remote_settings::client::RemoteSettingsClient;

lazy_static! {
    static ref NON_SPONSORED_IAB_CATEGORIES: Vec<&'static str> = vec!["5 - Education"];
}

/// Make suggestions based on data in Remote Settings
#[derive(Default)]
pub struct RemoteSettingsSuggester {
    /// A map from keywords to suggestions that can be provided.
    suggestions: HashMap<String, Arc<Suggestion>>,

    /// Client used for syncing
    remote_settings_client: Option<RemoteSettingsClient>,
}

impl RemoteSettingsSuggester {
    /// Make and sync a new suggester.
    ///
    /// # Errors
    pub async fn new_boxed(
        settings: &Settings,
        config: &RemoteSettingsConfig,
    ) -> Result<Box<Self>, SetupError> {
        let remote_settings_client = RemoteSettingsClient::new(
            &settings.remote_settings.server,
            config.bucket.clone(),
            config.collection.clone(),
        )?;
        let mut provider = Self {
            suggestions: HashMap::new(),
            remote_settings_client: Some(remote_settings_client),
        };
        provider.sync().await?;
        Ok(Box::new(provider))
    }

    /// Make a suggester for testing that only includes the given suggestions, and never syncs.
    #[cfg(test)]
    const fn with_suggestions(suggestions: HashMap<String, Arc<Suggestion>>) -> Self {
        Self {
            suggestions,
            remote_settings_client: None,
        }
    }

    /// Download suggestions from Remote Settings
    ///
    /// This must be called at least once before any suggestions will be provided
    #[tracing::instrument(skip(self))]
    pub async fn sync(&mut self) -> Result<(), SetupError> {
        tracing::info!(
            r#type = "adm.remote-settings.sync-start",
            "Syncing quicksuggest records from Remote Settings"
        );

        let rs_client = self.remote_settings_client.as_mut().ok_or_else(|| {
            SetupError::InvalidConfiguration(anyhow!(
                "Cannot sync without a remote_settings_client"
            ))
        })?;
        rs_client.sync().await?;

        // Download and process all the attachments concurrently
        let mut suggestion_attachments = futures::stream::FuturesUnordered::new();
        for record in rs_client.records_of_type("data".to_string()) {
            if let Some(attachment) = record.attachment() {
                tracing::trace!(?attachment.hash, "Queueing future to fetch attachment");
                suggestion_attachments.push(attachment.fetch::<Vec<AdmSuggestion>>());
            }
        }

        tracing::trace!("loading suggestions from records");

        // Build a map of icon IDs to URLs.
        let icon_urls: HashMap<_, _> = rs_client
            .records_of_type("icon".to_string())
            .filter_map(|record| {
                record
                    .attachment()
                    .map(|attachment| (&record.id, &attachment.location))
            })
            .collect();

        // Convert the collection of adM suggestion attachments into a lookup
        // table of keyword -> merino suggestion.
        let mut suggestions = HashMap::new();
        while let Some(attachment) = suggestion_attachments.next().await {
            let attachment = attachment?;
            tracing::trace!(?attachment, "processing attachment");
            for adm_suggestion in attachment {
                if adm_suggestion.keywords.is_empty() {
                    tracing::warn!(
                        ?adm_suggestion,
                        "Suggestion from remote settings has no keywords"
                    );
                    continue;
                }

                let icon_key = format!("icon-{}", adm_suggestion.icon);
                let icon_url = if let Some(u) = icon_urls.get(&icon_key) {
                    Uri::from_maybe_shared(u.to_string()).expect("invalid URL")
                } else {
                    tracing::warn!(suggestion_id = %adm_suggestion.id, "ADM suggestion has no icon");
                    continue;
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
                    impression_url: adm_suggestion.impression_url.clone(),
                    click_url: adm_suggestion.click_url.clone(),
                    full_keyword,
                    provider: adm_suggestion.advertiser.clone(),
                    is_sponsored: !NON_SPONSORED_IAB_CATEGORIES
                        .contains(&adm_suggestion.iab_category.as_str()),
                    icon: icon_url,
                    score: Proportion::from(0.2),
                });
                for keyword in &adm_suggestion.keywords {
                    suggestions.insert(keyword.clone(), merino_suggestion.clone());
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
    use std::{collections::HashMap, sync::Arc};

    use fake::{Fake, Faker};
    use http::Uri;
    use merino_suggest::{Proportion, Suggestion, SuggestionProvider, SuggestionRequest};

    use crate::remote_settings::RemoteSettingsSuggester;

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
        let rs_suggester = RemoteSettingsSuggester::with_suggestions(suggestions);

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
        let rs_suggester = RemoteSettingsSuggester::with_suggestions(suggestions);

        let request = SuggestionRequest {
            query: "sheep".into(),
            accepts_english: false,
            ..Faker.fake()
        };

        assert!(rs_suggester.suggest(request).await?.suggestions.is_empty());

        Ok(())
    }
}
