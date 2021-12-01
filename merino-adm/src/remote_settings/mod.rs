//! AdM integration that uses the remote-settings provided data.

pub mod client;

use crate::remote_settings::client::RemoteSettingsClient;
use async_trait::async_trait;
use cadence::{Histogrammed, StatsdClient};
use deduped_dashmap::DedupedMap;
use futures::StreamExt;
use http::Uri;
use lazy_static::lazy_static;
use merino_settings::{providers::RemoteSettingsConfig, Settings};
use merino_suggest::{
    CacheInputs, Proportion, SetupError, SuggestError, Suggestion, SuggestionProvider,
    SuggestionRequest, SuggestionResponse,
};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::{collections::HashMap, sync::Arc, time::Instant};

lazy_static! {
    static ref NON_SPONSORED_IAB_CATEGORIES: Vec<&'static str> = vec!["5 - Education"];
}

/// Make suggestions based on data in Remote Settings
pub struct RemoteSettingsSuggester {
    /// A map from keywords to suggestions that can be provided.
    suggestions: Arc<DedupedMap<String, (), Suggestion>>,

    /// A map from keywords to suggestions that can be provided.
    metrics_client: StatsdClient,
}

impl RemoteSettingsSuggester {
    /// Make and sync a new suggester.
    ///
    /// # Errors
    /// Returns an error if the settings are invalid for this provider, or if
    /// the initial sync fails.
    pub async fn new_boxed(
        settings: &Settings,
        config: &RemoteSettingsConfig,
        metrics_client: StatsdClient,
    ) -> Result<Box<Self>, SetupError> {
        let mut remote_settings_client = RemoteSettingsClient::new(
            &settings.remote_settings.server,
            config
                .bucket
                .as_ref()
                .unwrap_or(&settings.remote_settings.default_bucket)
                .clone(),
            config
                .collection
                .as_ref()
                .unwrap_or(&settings.remote_settings.default_collection)
                .clone(),
        )?;
        let suggestions = Arc::new(DedupedMap::new());

        Self::sync(&mut remote_settings_client, &*suggestions).await?;
        {
            let task_suggestions = Arc::clone(&suggestions);
            let task_interval = config.resync_interval;
            let mut task_client = Arc::new(remote_settings_client);

            tokio::spawn(async move {
                let mut timer = tokio::time::interval(task_interval);
                // The timer fires immediately, but we don't want to run the
                // sync function immediately, so wait one tick before starting
                // the loop.
                timer.tick().await;

                loop {
                    timer.tick().await;
                    let loop_suggestions = &*(Arc::clone(&task_suggestions));
                    if let Some(loop_client) = Arc::get_mut(&mut task_client) {
                        if let Err(error) = Self::sync(loop_client, loop_suggestions).await {
                            tracing::error!(
                                ?error,
                                "Error while syncing remote settings suggestions"
                            );
                        }
                    }
                }
            });
        }

        Ok(Box::new(Self {
            suggestions,
            metrics_client,
        }))
    }

    /// Make a suggester for testing that only includes the given suggestions, and never syncs.
    #[cfg(test)]
    fn with_suggestions(suggestions: HashMap<String, Suggestion>) -> Self {
        let deduped = suggestions.into_iter().collect();
        Self {
            suggestions: Arc::new(deduped),
            metrics_client: StatsdClient::builder("merino", cadence::NopMetricSink).build(),
        }
    }

    /// Download suggestions from Remote Settings
    ///
    /// This must be called at least once before any suggestions will be provided
    #[tracing::instrument(skip(remote_settings_client, suggestions))]
    pub async fn sync(
        remote_settings_client: &mut RemoteSettingsClient,
        suggestions: &DedupedMap<String, (), Suggestion>,
    ) -> Result<(), SetupError> {
        tracing::info!(
            r#type = "adm.remote-settings.sync-start",
            server = %remote_settings_client.server_url(),
            bucket = %remote_settings_client.bucket_id(),
            collection = %remote_settings_client.collection_id(),
            "Syncing quicksuggest records from Remote Settings"
        );
        remote_settings_client.sync().await?;

        // Download and process all the attachments concurrently
        let mut suggestion_attachments = futures::stream::FuturesUnordered::new();
        for record in remote_settings_client.records_of_type("data".to_string()) {
            if let Some(attachment) = record.attachment() {
                tracing::trace!(?attachment.hash, "Queueing future to fetch attachment");
                suggestion_attachments.push(async move {
                    (
                        attachment.hash.clone(),
                        attachment.fetch::<Vec<AdmSuggestion>>().await,
                    )
                });
            }
        }

        tracing::trace!("loading suggestions from records");

        // Build a map of icon IDs to URLs.
        let icon_urls: HashMap<_, _> = remote_settings_client
            .records_of_type("icon".to_string())
            .filter_map(|record| {
                record
                    .attachment()
                    .map(|attachment| (&record.id, &attachment.location))
            })
            .collect();

        // Convert the collection of adM suggestion attachments into a lookup
        // table of keyword -> merino suggestion.
        let mut new_suggestions = HashMap::new();
        while let Some((attachment_hash, attachment_content)) = suggestion_attachments.next().await
        {
            let attachment_suggestions = attachment_content?;
            tracing::trace!(%attachment_hash, suggestion_count = ?attachment_suggestions.len(), "processing attachment");
            for adm_suggestion in attachment_suggestions {
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

                let merino_suggestion = Suggestion {
                    id: adm_suggestion.id,
                    title: adm_suggestion.title.clone(),
                    url: adm_suggestion.url.clone(),
                    impression_url: Some(adm_suggestion.impression_url.clone()),
                    click_url: Some(adm_suggestion.click_url.clone()),
                    full_keyword: String::new(),
                    provider: adm_suggestion.advertiser.clone(),
                    is_sponsored: !NON_SPONSORED_IAB_CATEGORIES
                        .contains(&adm_suggestion.iab_category.as_str()),
                    icon: icon_url,
                    score: Proportion::from(0.2),
                };

                for keyword in &adm_suggestion.keywords {
                    let full_keyword = Self::get_full_keyword(keyword, &adm_suggestion.keywords);

                    new_suggestions.insert(
                        keyword.clone(),
                        Suggestion {
                            full_keyword,
                            ..merino_suggestion.clone()
                        },
                    );
                }
            }
        }

        if new_suggestions.is_empty() {
            tracing::warn!(
                r#type = "adm.remote-settings.empty",
                "No suggestion records found on Remote Settings"
            );
        }

        suggestions.retain(|_, _, _| deduped_dashmap::ControlFlow::Continue(false));
        for (k, v) in new_suggestions {
            suggestions.insert(k, (), v);
        }

        tracing::info!(
            r#type = "adm.remote-settings.sync-done",
            "Completed syncing quicksuggest records from Remote Settings"
        );

        Ok(())
    }

    /// Gets the "full keyword" (the suggested completion) for a query. The data
    /// from adM doesn't include this data directly, so we make our own based on
    /// the available keywords.
    ///
    /// 1. Find the first keyword phrase that has more words than the query. Use
    ///    its first `query_num_words` words as the full keyword. e.g., if the
    ///    query is `"moz"` and `all_keywords` is `["moz", "mozi", "mozil",
    ///    "mozill", "mozilla", "mozilla firefox"]`, pick `"mozilla firefox"`,
    ///    pop off the `"firefox"` and use `"mozilla"` as the full keyword.
    /// 2. If there isn't any keyword phrase with more words, then pick the
    ///    longest phrase. e.g., pick `"`mozilla" in the previous example
    ///    (assuming the `"mozilla firefox"` phrase isn't there). That might be
    ///    the query itself.
    ///
    fn get_full_keyword(partial_query: &str, all_keywords: &[String]) -> String {
        let query_num_words = partial_query.split_whitespace().count();

        // heuristic 1: more words
        if let Some(longer_keyword_words) = all_keywords
            .iter()
            .map(|keyword| keyword.split_whitespace().collect::<Vec<_>>())
            .find(|split_words| split_words.len() > query_num_words)
        {
            longer_keyword_words[..query_num_words].join(" ")
        } else {
            // heuristic 2 - longest phrase with partial query as a prefix
            all_keywords
                .iter()
                .filter(|keyword| keyword.starts_with(partial_query))
                .cloned()
                .max_by_key(String::len)
                .unwrap_or_else(|| partial_query.to_string())
        }
    }
}

#[async_trait]
impl SuggestionProvider for RemoteSettingsSuggester {
    fn name(&self) -> String {
        "AdmRemoteSettings".into()
    }

    fn cache_inputs(&self, req: &SuggestionRequest, cache_inputs: &mut dyn CacheInputs) {
        cache_inputs.add(&[req.accepts_english as u8]);
        cache_inputs.add(req.query.as_bytes());
    }

    async fn suggest(
        &self,
        request: SuggestionRequest,
    ) -> Result<SuggestionResponse, SuggestError> {
        let start = Instant::now();
        let suggestions = SuggestionResponse::new(if request.accepts_english {
            match self.suggestions.get(&request.query) {
                Some((_, suggestion)) => vec![suggestion],
                _ => vec![],
            }
        } else {
            vec![]
        });

        self.metrics_client
            .histogram_with_tags(
                "adm.rs.provider.duration-us",
                start.elapsed().as_micros() as u64,
            )
            .with_tag(
                "accepts-english",
                if request.accepts_english {
                    "true"
                } else {
                    "false"
                },
            )
            .send();

        Ok(suggestions)
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
#[derive(Clone, Debug, Deserialize, Serialize)]
#[allow(clippy::missing_docs_in_private_items)]
pub struct AdmSuggestion {
    /// Ad block ID
    pub id: u64,

    /// URL to direct the user to
    #[serde_as(as = "DisplayFromStr")]
    pub url: Uri,

    /// URL to notify if the user clicks on the suggestion
    #[serde_as(as = "DisplayFromStr")]
    pub click_url: Uri,

    /// URL to notify if the suggestion is shown toa user
    #[serde_as(as = "DisplayFromStr")]
    pub impression_url: Uri,

    /// Category of the suggestion (primarily sponsored or non-sponsored)
    pub iab_category: String,

    /// The ID of the icon (also stored in Remote Settings) to be used
    #[serde_as(as = "DisplayFromStr")]
    pub icon: u64,

    /// The advertiser this ad is from
    pub advertiser: String,

    /// The title of the suggestion to show to the user
    pub title: String,

    /// Keywords that can trigger this suggestion
    pub keywords: Vec<String>,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use fake::{Fake, Faker};
    use http::Uri;
    use merino_suggest::{Proportion, Suggestion, SuggestionProvider, SuggestionRequest};

    use crate::remote_settings::RemoteSettingsSuggester;

    #[actix_rt::test]
    async fn english_is_supported_example() -> anyhow::Result<()> {
        let mut suggestions = HashMap::new();
        suggestions.insert(
            "sheep".to_string(),
            Suggestion {
                title: "Wikipedia - Sheep".to_string(),
                url: Uri::from_static("https://en.wikipedia.org/wiki/Sheep"),
                id: 1,
                full_keyword: "sheep".to_string(),
                impression_url: Some(Uri::from_static("https://127.0.0.1")),
                click_url: Some(Uri::from_static("https://127.0.0.1")),
                provider: "test".to_string(),
                is_sponsored: false,
                icon: Uri::from_static("https://en.wikipedia.org/favicon.ico"),
                score: Proportion::zero(),
            },
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
            Suggestion {
                title: "Wikipedia - Sheep".to_string(),
                url: Uri::from_static("https://en.wikipedia.org/wiki/Sheep"),
                id: 1,
                full_keyword: "sheep".to_string(),
                impression_url: Some(Uri::from_static("https://127.0.0.1")),
                click_url: Some(Uri::from_static("https://127.0.0.1")),
                provider: "test".to_string(),
                is_sponsored: false,
                icon: Uri::from_static("https://en.wikipedia.org/favicon.ico"),
                score: Proportion::zero(),
            },
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

    #[test]
    fn get_full_keyword_matches_doc_heuristic_1() {
        assert_eq!(
            RemoteSettingsSuggester::get_full_keyword(
                "moz",
                &[
                    "moz".to_string(),
                    "mozi".to_string(),
                    "mozil".to_string(),
                    "mozill".to_string(),
                    "mozilla".to_string(),
                    "mozilla firefox".to_string(),
                ]
            ),
            "mozilla"
        );
        assert_eq!(
            RemoteSettingsSuggester::get_full_keyword(
                "one t",
                &[
                    "one".to_string(),
                    "one t".to_string(),
                    "one tw".to_string(),
                    "one two".to_string(),
                    "one two t".to_string(),
                    "one two th".to_string(),
                    "one two thr".to_string(),
                    "one two thre".to_string(),
                    "one two three".to_string(),
                ]
            ),
            "one two"
        );
    }

    #[test]
    fn get_full_keyword_matches_doc_heuristic_2() {
        assert_eq!(
            RemoteSettingsSuggester::get_full_keyword(
                "moz",
                &[
                    "moz".to_string(),
                    "mozi".to_string(),
                    "mozil".to_string(),
                    "mozill".to_string(),
                    "mozilla".to_string(),
                ]
            ),
            "mozilla"
        );
        assert_eq!(
            RemoteSettingsSuggester::get_full_keyword(
                "one two t",
                &[
                    "one".to_string(),
                    "one t".to_string(),
                    "one tw".to_string(),
                    "one two".to_string(),
                    "one two t".to_string(),
                    "one two th".to_string(),
                    "one two thr".to_string(),
                    "one two thre".to_string(),
                    "one two three".to_string(),
                ]
            ),
            "one two three"
        );
    }
}
