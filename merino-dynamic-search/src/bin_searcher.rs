use anyhow::{Context, Result};
use merino_dynamic_search::do_search;

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

    Ok(())
}
