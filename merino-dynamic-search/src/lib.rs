use anyhow::{anyhow, ensure, Result};
use futures::{stream::FuturesUnordered, StreamExt};
use merino_adm::remote_settings::{client::RemoteSettingsClient, AdmSuggestion};
use merino_settings::Settings;
use std::path::Path;
use tantivy::{
    collector::TopDocs, query::QueryParser, schema::Schema, Document, Index, IndexReader,
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

    let title_field = searcher
        .schema()
        .get_field("title")
        .ok_or_else(|| anyhow!("Missing title field"))?;
    let content_field = searcher
        .schema()
        .get_field("content")
        .ok_or_else(|| anyhow!("Missing content field"))?;

    let query_parser =
        QueryParser::for_index(reader.searcher().index(), vec![title_field, content_field]);
    let parsed_query = query_parser.parse_query(&query.replace('-', " "))?;

    searcher
        .search(&parsed_query, &TopDocs::with_limit(3))?
        .into_iter()
        .map(|(score, doc_address)| Ok((score, searcher.doc(doc_address)?)))
        .collect()
}
