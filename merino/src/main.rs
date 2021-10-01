//! A web API and associated tools to power Firefox Suggest.

#![warn(missing_docs, clippy::missing_docs_in_private_items)]

mod sentry;

use anyhow::{Context, Result};
use cadence::{BufferedUdpMetricSink, CountedExt, QueuingMetricSink, StatsdClient};
use merino_settings::{LogFormat, Settings};
use std::net::{TcpListener, UdpSocket};
use tracing::Level;
use tracing_actix_web_mozlog::{JsonStorageLayer, MozLogFormatLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{layer::SubscriberExt, EnvFilter};

/// Primary entry point
#[actix_rt::main]
async fn main() -> Result<()> {
    let settings = merino_settings::Settings::load().context("Loading settings")?;
    let _sentry_guard = crate::sentry::init_sentry(&settings).context("initializing sentry")?;
    init_logging(&settings).context("initializing logging")?;
    let metrics_client = init_metrics(&settings).context("initializing metrics")?;

    let listener = TcpListener::bind(settings.http.listen).context("Binding port")?;
    merino_web::run(listener, metrics_client, settings)
        .context("Starting merino-web server")?
        .await
        .context("Running merino-web server")?;

    Ok(())
}

/// Set up logging for Merino, based on settings and the `RUST_LOG` environment variable.
fn init_logging(settings: &Settings) -> Result<()> {
    LogTracer::init()?;
    let env_filter: EnvFilter = (&settings.logging.levels).into();

    match settings.logging.format {
        LogFormat::Pretty => {
            let subscriber = tracing_subscriber::FmtSubscriber::builder()
                .pretty()
                .with_max_level(Level::TRACE)
                .finish()
                .with(env_filter);
            tracing::subscriber::set_global_default(subscriber)?;
        }
        LogFormat::Compact => {
            let subscriber = tracing_subscriber::FmtSubscriber::builder()
                .with_level(true)
                .with_max_level(Level::TRACE)
                .finish()
                .with(env_filter);
            tracing::subscriber::set_global_default(subscriber)?;
        }
        LogFormat::MozLog => {
            let subscriber = tracing_subscriber::registry()
                .with(JsonStorageLayer)
                .with(MozLogFormatLayer::new("merino", std::io::stdout))
                .with(env_filter);
            tracing::subscriber::set_global_default(subscriber)?;
        }
    };

    let _span_guard = tracing::debug_span!("init_logging").entered();
    tracing::debug!("logging set up");

    Ok(())
}

#[tracing::instrument(level = "DEBUG", skip(settings))]
/// Set up metrics for Merino, based on settings.
fn init_metrics(settings: &Settings) -> Result<StatsdClient> {
    // We'll only be sending on this socket, so the host and port don't matter.
    let socket = UdpSocket::bind("0.0.0.0:0").context("creating metrics socket")?;
    socket
        .set_nonblocking(true)
        .context("setting metrics port to nonblocking")?;

    let queue_size = settings.metrics.max_queue_size_kb * 1024;

    // Make metrics show up immediately in development by using a non-buffered
    // sink. This would be a terrible idea in production though, so in
    // production use the buffered version. However, still use the queuing sink,
    // which is run on a different thread. This way we still get the concurrency
    // complexity, in case it causes bugs.
    let host = (
        settings.metrics.sink_host.as_str(),
        settings.metrics.sink_port,
    );
    let sink = if settings.debug {
        let udp_sink =
            cadence::UdpMetricSink::from(host, socket).context("setting up debug metrics sink")?;
        QueuingMetricSink::with_capacity(udp_sink, queue_size)
    } else {
        let udp_sink =
            BufferedUdpMetricSink::from(host, socket).context("setting up metrics sink")?;
        QueuingMetricSink::with_capacity(udp_sink, queue_size)
    };

    let client = StatsdClient::from_sink("merino", sink);

    // Test the newly made metrics client
    client
        .incr("startup")
        .context("Sending startup metrics ping")?;

    tracing::debug!(sink_host=?settings.metrics.sink_host, sink_port=?settings.metrics.sink_port, ?queue_size, "metrics set up");
    Ok(client)
}

#[cfg(test)]
mod tests {
    use crate::*;

    #[test]
    fn test_metrics_sink() {
        let mut s = Settings::load_for_tests();

        // Use an invalid hostname to make sure things break.
        s.metrics.sink_host = "#invalid-hostname".to_string();
        s.metrics.sink_port = 0;
        assert!(init_metrics(&s).is_err());

        // Use a hostname to set up the metrics sink.
        s.metrics.sink_host = "localhost".to_string();
        s.metrics.sink_port = 0;
        assert!(init_metrics(&s).is_ok());

        // Use an IP to set up the metrics sink.
        s.metrics.sink_host = "127.0.0.1".to_string();
        s.metrics.sink_port = 0;
        assert!(init_metrics(&s).is_ok());
    }
}
