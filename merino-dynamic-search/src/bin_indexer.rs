use anyhow::{anyhow, Result};
use merino_dynamic_search::get_wiki_suggestions;
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

    println!("Loading page contents from Wikipedia API");
    let pages = get_wiki_texts(&wiki_titles.iter().map(String::as_str).collect::<Vec<_>>()).await?;

    println!("Indexing page contents into Tantivy");
    let index = merino_dynamic_search::get_search_index()?;

    let mut index_writer = index.writer(50 * MEGA)?;
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

    for page in pages {
        index_writer.add_document(doc!(
            title_field => page.title,
            content_field => page.content,
            url_field => page.url,
        ));
    }

    index_writer.commit()?;

    println!("Done");
    Ok(())
}

#[derive(Clone, Debug)]
struct PageToIndex {
    title: String,
    url: String,
    content: String,
}

async fn get_wiki_texts(page_titles: &[&str]) -> Result<Vec<PageToIndex>> {
    let client = reqwest::Client::new();
    let mut rv = Vec::with_capacity(page_titles.len());

    for chunk in page_titles.chunks(50) {
        let titles_concat = chunk.join("|");
        let url = format!("https://en.wikipedia.org/w/api.php?action=query&prop=revisions&titles={}&rvslots=main&rvprop=content&formatversion=2&format=json&redirects=1", titles_concat);

        let res = client
            .get(url)
            .header("Content-Type", "application/json")
            .send()
            .await?
            .error_for_status()?;

        let data: serde_json::Value = res.json().await?;
        rv.extend_from_slice(
            data["query"]["pages"]
                .as_array()
                .ok_or_else(|| anyhow!("Could not get list of pages from Wikipedia response"))?
                .iter()
                .map(|page| {
                    let title = page["title"]
                        .as_str()
                        .ok_or_else(|| anyhow!("no title"))?
                        .to_string();
                    let url = format!("https://en.wikipedia.org/wiki/{}", title.replace(" ", "_"));
                    Ok(PageToIndex {
                        url,
                        content: page["revisions"][0]["slots"]["main"]["content"]
                            .as_str()
                            .ok_or_else(|| anyhow!(format!("No content for {}", title)))?
                            .to_string(),
                        title,
                    })
                })
                .collect::<Result<Vec<_>>>()?
                .as_slice(),
        );
    }

    Ok(rv)
}
