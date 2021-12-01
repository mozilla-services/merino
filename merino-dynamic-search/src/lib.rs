use anyhow::{anyhow, ensure, Context, Result};
use async_trait::async_trait;
use futures::{stream::FuturesUnordered, StreamExt};
use merino_adm::remote_settings::{client::RemoteSettingsClient, AdmSuggestion};
use merino_settings::providers::TantivyConfig;
use merino_settings::Settings;
use merino_suggest::{Proportion, SetupError, SuggestError, Suggestion, SuggestionResponse};
use std::convert::TryInto;
use std::path::Path;
use tantivy::schema::Value;
use tantivy::{
    collector::TopDocs,
    query::{BooleanQuery, BoostQuery, Occur, QueryParser},
    schema::Schema,
    Document, Index, IndexReader,
};

/// Get the index for the search engine
/// # Errors
/// If tantivy is unhappy
pub fn get_search_index() -> Result<Index> {
    let index_path = Path::new("./tantivy-index");
    std::fs::create_dir_all(index_path)?;

    let schema = {
        use tantivy::schema::{STORED, TEXT};
        let mut builder = Schema::builder();
        // TEXT = tokenized and available to search
        // STORED = stored in the index to be retrieved later
        builder.add_text_field("title", STORED | TEXT);
        builder.add_text_field("content", TEXT);
        builder.add_text_field("url", STORED);
        builder.add_text_field("page_id", STORED);
        builder.build()
    };

    let index = Index::open_in_dir(&index_path).or_else(|err| {
        println!("warn: {}", err);
        Index::create_in_dir(index_path, schema.clone())
    })?;
    ensure!(index.schema() == schema);

    Ok(index)
}

/// Get all the Wikipedia suggestions from the Remote Settings collection.
/// # Errors
/// If something gets mad.
pub async fn get_wiki_suggestions() -> Result<Vec<AdmSuggestion>> {
    let Settings {
        remote_settings, ..
    } = Settings::load()?;

    println!("Loading list of Wikipedia pages from Remote Settings");
    let mut remote_settings_client = RemoteSettingsClient::new(
        &remote_settings.server,
        remote_settings.default_bucket,
        remote_settings.default_collection,
    )?;
    remote_settings_client.sync().await?;

    // Download and process all the attachments concurrently
    let mut suggestion_attachments = FuturesUnordered::new();
    for record in remote_settings_client.records_of_type("data".to_string()) {
        if let Some(attachment) = record.attachment() {
            suggestion_attachments.push(attachment.fetch::<Vec<AdmSuggestion>>());
        }
    }

    let mut rv = Vec::new();
    while let Some(attachment) = suggestion_attachments.next().await {
        rv.extend(
            attachment?
                .into_iter()
                .filter(|s| s.advertiser == "Wikipedia"),
        );
    }

    Ok(rv)
}

/// Perform a standardized search on a reader.
/// # Errors
/// If the searcher is misconfigured or if the query is not in the expected format.
pub fn do_search(reader: &IndexReader, query: &str) -> Result<Vec<(f32, Document)>> {
    let searcher = reader.searcher();

    let query = query.replace('-', " ");

    let title_field = searcher
        .schema()
        .get_field("title")
        .ok_or_else(|| anyhow!("Missing title field"))?;
    let content_field = searcher
        .schema()
        .get_field("content")
        .ok_or_else(|| anyhow!("Missing content field"))?;

    let title_query_parser = QueryParser::for_index(reader.searcher().index(), vec![title_field]);
    let content_query_parser =
        QueryParser::for_index(reader.searcher().index(), vec![content_field]);
    let built_query = BooleanQuery::new(vec![
        (
            Occur::Should,
            Box::new(BoostQuery::new(
                Box::new(title_query_parser.parse_query(&query)?),
                3.0,
            )),
        ),
        (
            Occur::Should,
            Box::new(content_query_parser.parse_query(&query)?),
        ),
    ]);

    searcher
        .search(&built_query, &TopDocs::with_limit(3))?
        .into_iter()
        .map(|(score, doc_address)| Ok((score, searcher.doc(doc_address)?)))
        .collect()
}

pub struct TantivyProvider {
    index_reader: IndexReader,
    threshold: f32,
}

impl TantivyProvider {
    /// Make a boxed provider
    /// # Errors
    /// If the search index could not be opened
    pub fn new_boxed(config: &TantivyConfig) -> Result<Box<Self>, SetupError> {
        let index = get_search_index()
            .context("Loading tantivity index for searching")
            .map_err(SetupError::Io)?;
        let index_reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::OnCommit)
            .try_into()
            .context("Setting up search reader")
            .map_err(SetupError::Io)?;

        Ok(Box::new(Self {
            index_reader,
            threshold: config.threshold,
        }))
    }
}

#[async_trait]
impl merino_suggest::SuggestionProvider for TantivyProvider {
    fn name(&self) -> String {
        "tantivity".to_string()
    }

    async fn suggest(
        &self,
        query: merino_suggest::SuggestionRequest,
    ) -> Result<merino_suggest::SuggestionResponse, merino_suggest::SuggestError> {
        macro_rules! get_field {
            ($field: expr) => {
                self.index_reader
                    .searcher()
                    .index()
                    .schema()
                    .get_field($field)
                    .context(concat!("Getting ", $field, " field"))
                    .map_err(SuggestError::Internal)
            };
        }

        let page_id_field = get_field!("page_id")?;
        let title_field = get_field!("title")?;
        let url_field = get_field!("url")?;

        macro_rules! field_value {
            ($doc: expr, $field: expr, $variant: expr) => {
                $doc.get_first($field)
                    .and_then($variant)
                    .ok_or_else(|| anyhow!("Invalid schema, no {}", stringify!($field)))
                    .map_err(SuggestError::Internal)
            };
        }

        let suggestions = do_search(&self.index_reader, &query.query)
            .map_err(SuggestError::Internal)?
            .into_iter()
            .filter(|(score, _doc)| *score > self.threshold)
            .map(|(score, doc)| {
                // map from [threshold, threshold * 3] to [0.4, 0.5]
                let range = self.threshold * 2.0_f32;
                let adjusted_score =
                    (score - self.threshold).min(range) / range / 10.0_f32 + 0.4_f32;

                dbg!(score, adjusted_score);
                debug_assert!((0.4..=0.5).contains(&adjusted_score));

                Ok(Suggestion {
                    id: field_value!(doc, page_id_field, Value::u64_value)?,
                    full_keyword: query.query.clone(),
                    title: format!(
                        "{} (score: {})",
                        field_value!(doc, title_field, Value::text)?,
                        score
                    ),
                    url: field_value!(doc, url_field, Value::text)?
                        .try_into()
                        .unwrap(),
                    impression_url: None,
                    click_url: None,
                    provider: "wiki-search".to_string(),
                    is_sponsored: false,
                    icon: http::Uri::from_static(
                        "https://en.wikipedia.org/static/apple-touch/wikipedia.png",
                    ),
                    score: Proportion::clamped(adjusted_score),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(SuggestionResponse::new(suggestions))
    }
}
