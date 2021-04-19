// Only overview documentation that is not relevant to one of the more specific
// crates should go here.

#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! A web API and associated tools to power Firefox Suggest.
//!
//! Merino is split into several subcrates that work in collaboration.
//!
//! - [merino-adm](../merino_adm/index.html)
//! - [merino-suggest](../merino_suggest/index.html)
//! - [merino-web](../merino_web/index.html)

use anyhow::{Context, Result};
use std::net::TcpListener;
use viaduct_reqwest::ReqwestBackend;

/// Primary entry point
#[actix_rt::main]
async fn main() -> Result<()> {
    viaduct::set_backend(&ReqwestBackend).context("setting viaduct backend")?;
    let listener = TcpListener::bind("0.0.0.0:8080").context("Binding port")?;
    merino_web::run(listener)
        .context("Starting merino-web server")?
        .await
        .context("Running merino-web server")?;

    Ok(())
}
