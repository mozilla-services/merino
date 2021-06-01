// Only overview documentation that is not relevant to one of the more specific
// crates should go here.

#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! A web API and associated tools to power Firefox Suggest.
//!
//! Merino is split into several subcrates that work in collaboration.
//!
//! - [merino-adm](../merino_adm/index.html)
//! - [merino-integration-tests](../merino_integration_tests/index.html)
//! - [merino-settings](../merino_settings/index.html)
//! - [merino-suggest](../merino_suggest/index.html)
//! - [merino-web](../merino_web/index.html)

mod docs;

use anyhow::{Context, Result};
use merino_settings::Settings;
use std::net::TcpListener;
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};
use viaduct_reqwest::ReqwestBackend;

/// Primary entry point
#[actix_rt::main]
async fn main() -> Result<()> {
    let settings = merino_settings::Settings::load().context("Loading settings")?;
    init_logging(&settings)?;
    viaduct::set_backend(&ReqwestBackend).context("setting viaduct backend")?;
    let listener = TcpListener::bind(settings.http.listen).context("Binding port")?;

    merino_web::run(listener, settings)
        .context("Starting merino-web server")?
        .await
        .context("Running merino-web server")?;

    Ok(())
}

/// Set up logging for Merino, based on settings and the `RUST_LOG` environment variable.
fn init_logging(settings: &Settings) -> Result<()> {
    LogTracer::init()?;
    let env_filter: EnvFilter = (&settings.logging.levels).into();

    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .pretty()
        .finish()
        .with(env_filter);

    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}
