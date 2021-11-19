use anyhow::{anyhow, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use merino_dynamic_search::get_wiki_suggestions;
use rayon::prelude::*;
use serde::Serialize;
use std::{fs, path::Path};
use tantivy::{collector::TopDocs, query::QueryParser};

#[tokio::main]
async fn main() -> Result<()> {
    println!("Syncing data from Remote Settings");
    let adm_suggestions = get_wiki_suggestions().await?;

    println!("Checking ADM keywords vs Tantivy");
    let index = merino_dynamic_search::get_search_index()?;
    let reader = index
        .reader_builder()
        .reload_policy(tantivy::ReloadPolicy::Manual)
        .try_into()
        .context("Setting up search reader")?;

    let title_field = index
        .schema()
        .get_field("title")
        .ok_or_else(|| anyhow!("Missing title field"))?;
    let content_field = index
        .schema()
        .get_field("content")
        .ok_or_else(|| anyhow!("Missing content field"))?;

    let query_parser = QueryParser::for_index(&index, vec![title_field, content_field]);

    let bar = ProgressBar::new(
        adm_suggestions
            .iter()
            .map(|s| s.keywords.len() as u64)
            .sum(),
    );
    bar.set_style(
        ProgressStyle::default_bar().template("[{eta}] {bar:40.cyan/blue} {pos:>6}/{len:6} {msg}"),
    );

    let pairs = adm_suggestions
        .into_iter()
        .flat_map(|suggestion| {
            let mut rv = Vec::with_capacity(suggestion.keywords.len());
            for keyword in &suggestion.keywords {
                rv.push((suggestion.clone(), keyword.clone()));
            }
            rv
        })
        .collect::<Vec<_>>();

    let vs_results = pairs
        .into_par_iter()
        .map(|(suggestion, keyword)| {
            bar.set_message(keyword.clone());
            bar.inc(1);

            let searcher = reader.searcher();
            let query = query_parser
                .parse_query(&keyword.replace("-", " "))
                .context(format!("invalid query {}", keyword))?;

            let search_results = searcher.search(&query, &TopDocs::with_limit(1))?;

            let (vs_adm, score) = match search_results.first() {
                None => (VsAdm::NoResult, None),
                Some((score, doc_address)) => {
                    let doc = searcher.doc(*doc_address)?;
                    let title = doc
                        .get_first(title_field)
                        .and_then(tantivy::schema::Value::text)
                        .ok_or_else(|| anyhow!("Invalid schema, no title"))?;
                    let vs_adm = if format!("Wikipedia - {}", title) == suggestion.title {
                        VsAdm::Match
                    } else {
                        VsAdm::NoMatch
                    };
                    (vs_adm, Some(*score))
                }
            };

            Ok(Output {
                suggestion_id: suggestion.id,
                suggestion_title: suggestion.title.clone(),
                keyword,
                vs_adm,
                score,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    // bar.finish_and_clear();

    let output_path = Path::new("./vs-output.json");
    println!(
        "Writing results to {}",
        output_path
            .to_str()
            .ok_or_else(|| anyhow!("non utf8 output path"))?
    );
    let mut output_file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(output_path)?;
    serde_json::to_writer(&mut output_file, &vs_results)?;

    println!("Done");
    Ok(())
}

#[derive(Serialize)]
struct Output {
    suggestion_id: u32,
    suggestion_title: String,
    keyword: String,
    vs_adm: VsAdm,
    score: Option<f32>,
}

#[derive(Serialize)]
enum VsAdm {
    Match,
    NoMatch,
    NoResult,
}
