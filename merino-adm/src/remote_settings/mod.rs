//! AdM integration that uses the remote-settings provided data.

mod reqwest_client;

use crate::remote_settings::reqwest_client::ReqwestClient;
use anyhow::Context;
use async_trait::async_trait;
use cadence::StatsdClient;
use deduped_dashmap::DedupedMap;
use http::Uri;
use lazy_static::lazy_static;
use merino_settings::{providers::RemoteSettingsConfig, Settings};
use merino_suggest::{
    metrics::TimedMicros, CacheInputs, Proportion, SetupError, SuggestError, Suggestion,
    SuggestionProvider, SuggestionRequest, SuggestionResponse,
};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};
use std::{collections::HashMap, convert::TryInto, sync::Arc, time::Instant};

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
        let mut remote_settings_client = remote_settings_client::Client::builder()
            .bucket_name(
                config
                    .bucket
                    .as_ref()
                    .unwrap_or(&settings.remote_settings.default_bucket)
                    .clone(),
            )
            .collection_name(
                config
                    .collection
                    .as_ref()
                    .unwrap_or(&settings.remote_settings.default_collection)
                    .clone(),
            )
            .server_url(&settings.remote_settings.server)
            .sync_if_empty(true)
            .storage(Box::new(remote_settings_client::client::FileStorage {
                folder: std::env::temp_dir(),
                ..remote_settings_client::client::FileStorage::default()
            }))
            .http_client(Box::new(ReqwestClient::new()))
            .build()
            .context("Unable to initialize the Remote Settings client")
            .map_err(SetupError::InvalidConfiguration)?;

        let suggestions = Arc::new(DedupedMap::new());
        let suggestion_score = config
            .suggestion_score
            .try_into()
            .context("converting score to proportion")
            .map_err(SetupError::InvalidConfiguration)?;

        Self::sync(&mut remote_settings_client, &*suggestions, suggestion_score).await?;

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
                        if let Err(error) =
                            Self::sync(loop_client, loop_suggestions, suggestion_score).await
                        {
                            tracing::error!(
                                r#type = "adm.remote-settings.sync-failed",
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
        remote_settings_client: &mut remote_settings_client::Client,
        suggestions: &DedupedMap<String, (), Suggestion>,
        suggestion_score: Proportion,
    ) -> Result<(), SetupError> {
        tracing::info!(
            r#type = "adm.remote-settings.sync-start",
            "Syncing quicksuggest records from Remote Settings"
        );

        let collection = remote_settings_client
            .sync(None)
            .await
            .context("Fetching records from remote settings")
            .map_err(SetupError::Network)?;

        let mut records: Vec<remote_settings_client::Record> = collection
            .records
            .into_iter()
            .filter(|r| !r.deleted())
            .collect();

        /// Used to parse the attachments coming from RS.
        #[derive(Debug, Deserialize)]
        #[serde(transparent)]
        struct VecAdm {
            /// The attachments coming from RS are in the shape
            /// of a top level JSON vector, so reflect that with
            /// this field. Note that, due to serde's 'transparent'
            /// property, this gets interpreted as a top level field.
            suggestions: Vec<AdmSuggestion>,
        }

        impl std::convert::TryFrom<Vec<u8>> for VecAdm {
            type Error = serde_json::Error;

            fn try_from(value: Vec<u8>) -> Result<Self, Self::Error> {
                serde_json::from_slice(&value)
            }
        }

        // Download and process all the attachments.
        let mut data_records = records.clone();
        data_records.retain(|r| r.get("type") == Some(&serde_json::json!("data")));

        tracing::trace!("loading suggestions from records");

        let server_info = remote_settings_client
            .server_info()
            .await
            .context("Fetching server information")
            .map_err(SetupError::Network)?;
        let attachments_base_url = match &server_info["capabilities"]["attachments"]["base_url"] {
            serde_json::Value::String(s) => s,
            _ => {
                return Err(SetupError::Format(anyhow::anyhow!(
                    "Could not get attachments base URL"
                )))
            }
        };

        // Build a map of icon IDs to URLs.
        records.retain(|r| r.get("type") == Some(&serde_json::json!("icon")));
        let mut icon_urls = HashMap::new();
        for mut record in records {
            let id = record.id().to_string();
            if let Ok(Some(meta)) = record.attachment_metadata() {
                let url = format!("{}{}", attachments_base_url, meta.location.clone());
                icon_urls.entry(id).or_insert(url);
            }
        }

        // Convert the collection of adM suggestion attachments into a lookup
        // table of keyword -> merino suggestion.
        let mut new_suggestions = HashMap::new();
        for record in data_records.iter_mut() {
            if let Ok(Some(parsed_data)) = remote_settings_client
                .fetch_attachment::<VecAdm, serde_json::Error>(record)
                .await
            {
                let attachment_hash = match record.attachment_metadata() {
                    Ok(Some(meta)) => &meta.hash,
                    _ => continue,
                };

                tracing::trace!(%attachment_hash, suggestion_count = ?parsed_data.suggestions.len(), "processing attachment");
                for adm_suggestion in parsed_data.suggestions {
                    if adm_suggestion.keywords.is_empty() {
                        tracing::warn!(
                            r#type = "adm.remote-settings.sync-no-keywords",
                            ?adm_suggestion,
                            "Suggestion from remote settings has no keywords"
                        );
                        continue;
                    }

                    let icon_key = format!("icon-{}", adm_suggestion.icon);
                    let icon_url = if let Some(u) = icon_urls.get(&icon_key) {
                        Uri::from_maybe_shared(u.to_string()).expect("invalid URL")
                    } else {
                        tracing::warn!(
                            r#type = "adm.remote-settings.sync-no-icon",
                            suggestion_id = %adm_suggestion.id, "ADM suggestion has no icon");
                        continue;
                    };

                    let merino_suggestion = Suggestion {
                        id: adm_suggestion.id,
                        title: adm_suggestion.title.clone(),
                        url: adm_suggestion.url.clone(),
                        impression_url: adm_suggestion.impression_url.clone(),
                        click_url: adm_suggestion.click_url.clone(),
                        full_keyword: String::new(),
                        provider: adm_suggestion.advertiser.clone(),
                        advertiser: adm_suggestion.advertiser.clone(),
                        is_sponsored: !NON_SPONSORED_IAB_CATEGORIES
                            .contains(&adm_suggestion.iab_category.as_str()),
                        icon: icon_url,
                        score: suggestion_score,
                    };

                    for keyword in &adm_suggestion.keywords {
                        let full_keyword =
                            Self::get_full_keyword(keyword, &adm_suggestion.keywords);

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
    /// 1. Filter out keywords that don't start with the value of `partial_query`.
    /// 2. Find the first keyword phrase that has more words than the query. Use
    ///    its first `query_num_words` words as the full keyword. e.g., if the
    ///    query is `"moz"` and `all_keywords` is `["moz", "mozi", "mozil",
    ///    "mozill", "mozilla", "mozilla firefox"]`, pick `"mozilla firefox"`,
    ///    pop off the `"firefox"` and use `"mozilla"` as the full keyword.
    /// 3. If there isn't any keyword phrase with more words, then pick the
    ///    longest phrase. e.g., pick `"`mozilla" in the previous example
    ///    (assuming the `"mozilla firefox"` phrase isn't there). That might be
    ///    the query itself.
    ///
    fn get_full_keyword(partial_query: &str, all_keywords: &[String]) -> String {
        let query_num_words = partial_query.split_whitespace().count();

        // heuristic 1: more words
        if let Some(longer_keyword_words) = all_keywords
            .iter()
            .filter(|keyword| keyword.starts_with(partial_query))
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
            .time_micros_with_tags("adm.rs.provider.duration-us", start.elapsed())
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
                impression_url: Uri::from_static("https://127.0.0.1"),
                click_url: Uri::from_static("https://127.0.0.1"),
                provider: "test".to_string(),
                advertiser: "test_advertiser".to_string(),
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
                impression_url: Uri::from_static("https://127.0.0.1"),
                click_url: Uri::from_static("https://127.0.0.1"),
                provider: "test".to_string(),
                advertiser: "test_advertiser".to_string(),
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
                "moz",
                &[
                    "gozilla".to_string(),
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
        assert_eq!(
            RemoteSettingsSuggester::get_full_keyword(
                "one",
                &[
                    "two".to_string(),
                    "two three".to_string(),
                    "one".to_string(),
                    "one t".to_string(),
                    "one tw".to_string(),
                    "one two".to_string(),
                ]
            ),
            "one"
        );
        assert_eq!(
            RemoteSettingsSuggester::get_full_keyword(
                "one t",
                &[
                    "two".to_string(),
                    "two three".to_string(),
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
                "moz",
                &[
                    "gozilla".to_string(),
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
        assert_eq!(
            RemoteSettingsSuggester::get_full_keyword(
                "one two t",
                &[
                    "two".to_string(),
                    "two three".to_string(),
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
