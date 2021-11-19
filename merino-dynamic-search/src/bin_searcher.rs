use anyhow::{anyhow, Context, Result};
use tantivy::{collector::TopDocs, query::QueryParser};

fn main() -> Result<()> {
    let search_query = std::env::args()
        .skip(1)
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    if search_query.is_empty() {
        eprintln!(
            "USAGE: {} search query terms",
            std::env::args().next().unwrap()
        );
        std::process::exit(1);
    }

    let index = merino_dynamic_search::get_search_index()?;
    let reader = index
        .reader_builder()
        .reload_policy(tantivy::ReloadPolicy::Manual)
        .try_into()
        .context("Setting up search reader")?;
    let searcher = reader.searcher();

    let title_field = index
        .schema()
        .get_field("title")
        .ok_or_else(|| anyhow!("Missing title field"))?;
    let content_field = index
        .schema()
        .get_field("content")
        .ok_or_else(|| anyhow!("Missing content field"))?;

    let query_parser = QueryParser::for_index(&index, vec![title_field, content_field]);
    let query = query_parser.parse_query(&search_query)?;

    let top_docs = searcher.search(&query, &TopDocs::with_limit(10))?;
    for (score, doc_address) in top_docs {
        let doc = searcher.doc(doc_address)?;
        println!("{} - {}", score, index.schema().to_json(&doc));
    }

    Ok(())
}
