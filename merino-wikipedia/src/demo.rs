use anyhow::{Context, Result};
use merino_wikipedia::{ElasticHelper, WikipediaDocument, WikipediaNamespace};

#[tokio::main]
async fn main() -> Result<()> {
    let settings = merino_settings::Settings::load()
        .await
        .context("Loading settings")?;

    let es_client = ElasticHelper::new(&settings.elasticsearch, "merino-scratch")?;

    es_client.index_ensure_exists().await?;

    let doc = WikipediaDocument {
        title: "A Test Page".to_string(),
        page_text: "There is some stuff here, probably.".to_string(),
        namespace: WikipediaNamespace::Article,
        page_id: 0,
    };

    es_client.doc_add(doc).await?;

    Ok(())
}
