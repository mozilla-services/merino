use std::{
    collections::HashMap,
    io::{BufReader, BufWriter, ErrorKind},
    path::PathBuf,
};

use anyhow::{anyhow, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use merino_dynamic_search::get_wiki_suggestions;
use serde::{Deserialize, Serialize};
use tantivy::doc;

const MEGA: usize = 1024 * 1024;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Syncing data from Remote Settings");
    let wiki_titles = get_wiki_suggestions()
        .await?
        .into_iter()
        .map(|adm_suggestion| {
            adm_suggestion
                .title
                .trim_start_matches("Wikipedia -")
                .trim()
                .to_string()
        })
        .collect::<Vec<_>>();

    let pages = get_wiki_texts(&wiki_titles.iter().map(String::as_str).collect::<Vec<_>>()).await?;

    println!("Indexing page contents into Tantivy");
    let bar = ProgressBar::new(pages.len() as u64);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed:>3}/{duration}] {bar:40.cyan/blue} {pos:>6}/{len:6} {wide_msg}"),
    );

    let index = merino_dynamic_search::get_search_index()?;
    let mut index_writer = index.writer(50 * MEGA)?;
    index_writer.delete_all_documents()?;

    let title_field = index
        .schema()
        .get_field("title")
        .ok_or_else(|| anyhow!("Missing title field"))?;
    let content_field = index
        .schema()
        .get_field("content")
        .ok_or_else(|| anyhow!("Missing content field"))?;
    let url_field = index
        .schema()
        .get_field("url")
        .ok_or_else(|| anyhow!("Missing url field"))?;
    let page_id_field = index
        .schema()
        .get_field("page_id")
        .ok_or_else(|| anyhow!("Missing page_id_field field"))?;

    for page in pages {
        bar.set_message(page.title.clone());
        bar.inc(1);

        index_writer.add_document(doc!(
            title_field => page.title,
            content_field => page.content,
            url_field => page.url,
            page_id_field => page.page_id,
        ));
    }

    bar.set_message("committing...");
    bar.tick();

    index_writer.commit()?;
    bar.set_message("");
    bar.finish();

    println!("Done");
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PageToIndex {
    title: String,
    url: String,
    content: String,
    page_id: u64,
}

async fn get_wiki_texts(page_titles: &[&str]) -> Result<Vec<PageToIndex>> {
    const CHUNK_SIZE: usize = 50;
    let client = reqwest::Client::new();
    let bar = ProgressBar::new(page_titles.len() as u64);
    bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed:>3}/{duration}] {bar:40.cyan/blue} {pos:>6}/{len:6} {wide_msg}"),
    );

    let mut pages_by_title: HashMap<&str, PageToIndex> = HashMap::new();

    bar.println("Loading page contents from cache");
    for title in page_titles {
        bar.set_message((*title).to_string());
        if let Some(page) = load_page_from_cache(*title)? {
            pages_by_title.insert(title, page);
            bar.inc(1);
        }
    }

    bar.println(format!("Loaded {} pages from cache", pages_by_title.len()));
    let pages_to_download: Vec<&str> = page_titles
        .iter()
        .copied()
        .filter(|title| !pages_by_title.contains_key(*title))
        .collect();

    if !pages_to_download.is_empty() {
        bar.println("Switching to Wikipedia API");
    }

    for chunk in pages_to_download.chunks(CHUNK_SIZE) {
        let titles_concat = chunk.join("|");
        bar.set_message(titles_concat.clone());
        let url = format!("https://en.wikipedia.org/w/api.php?action=query&prop=revisions&titles={}&rvslots=main&rvprop=content&formatversion=2&format=json&redirects=1", titles_concat);

        let res = client
            .get(url)
            .header("Content-Type", "application/json")
            .send()
            .await?
            .error_for_status()?;

        std::fs::create_dir_all("./wikipedia-page-cache")
            .map_err(|err| bar.println(format!("Warn: Could not create cache dir: {}", err)))
            .ok();

        let data: serde_json::Value = res.json().await?;
        data["query"]["pages"]
            .as_array()
            .ok_or_else(|| anyhow!("Could not get list of pages from Wikipedia response"))?
            .iter()
            .zip(chunk)
            .map::<Result<(&str, PageToIndex)>, _>(|(page, original_title)| {
                let wiki_title = page["title"]
                    .as_str()
                    .ok_or_else(|| anyhow!("no title"))?
                    .to_string();
                let url = format!(
                    "https://en.wikipedia.org/wiki/{}",
                    wiki_title.replace(" ", "_")
                );
                Ok((
                    original_title,
                    PageToIndex {
                        url,
                        page_id: page["pageid"]
                            .as_u64()
                            .ok_or_else(|| anyhow!(format!("No page_id for {}", wiki_title)))?,
                        content: page["revisions"][0]["slots"]["main"]["content"]
                            .as_str()
                            .ok_or_else(|| anyhow!(format!("No content for {}", wiki_title)))?
                            .to_string(),
                        title: wiki_title,
                    },
                ))
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|(original_title, page)| {
                save_page_to_cache(&page)
                    .map_err(|err| {
                        bar.println(format!(
                            "Error: Could not save downloaded page to cache: {}",
                            err
                        ));
                    })
                    .ok();

                pages_by_title.insert(original_title, page);

                bar.inc(1);
                Ok(())
            })
            .collect::<Result<Vec<()>, anyhow::Error>>()?;
    }

    bar.set_message("");
    bar.finish();

    Ok(pages_by_title.into_values().collect())
}

fn cache_path(title: &str) -> PathBuf {
    format!("./wikipedia-page-cache/{}.json", title).into()
}

fn load_page_from_cache(title: &str) -> Result<Option<PageToIndex>> {
    let file_path = cache_path(title);
    match std::fs::File::open(&file_path) {
        Ok(file) => {
            let buffered = BufReader::new(file);
            serde_json::from_reader(buffered)
                .map_err(|err| {
                    std::fs::remove_file(&file_path).ok();
                    err
                })
                .context("deserializing cached page")
        }
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => Err(anyhow::Error::new(err)),
    }
}

fn save_page_to_cache(page: &PageToIndex) -> Result<()> {
    let file_path = cache_path(&page.title);
    let f = std::fs::File::create(file_path)?;
    let buffered = BufWriter::new(f);
    serde_json::to_writer(buffered, page)?;
    Ok(())
}
