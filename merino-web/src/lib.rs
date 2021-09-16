#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! Web server for [Merino](../merino/index.html)'s public API.

mod debug;
mod dockerflow;
mod errors;
mod extractors;
mod middleware;
mod suggest;

use actix_cors::Cors;
use actix_web::{
    dev::Server,
    get,
    web::{self, Data},
    App, HttpResponse, HttpServer,
};
use actix_web_location::{providers::FallbackProvider, Location};
use anyhow::Context;
use cadence::StatsdClient;
use merino_settings::Settings;
use std::net::TcpListener;
use tracing_actix_web_mozlog::MozLog;

/// Run the web server
///
/// The returned server is a `Future` that must either be `.await`ed, or run it
/// as a background task using `tokio::spawn`.
///
/// Most of the details from `settings` will be respected, except for those that
/// go into building the listener (the host and port). If you want to respect the
/// settings specified in that object, you must include them in the construction
/// of `listener`.
///
/// # Context
///
/// The code initialized here emits logs and metrics, assuming global defaults
/// have been set. Logs are emitted via [`tracing`], and metrics via
/// [`cadence`].
///
/// # Errors
///
/// Returns an error if the server cannot be started on the provided listener.
///
/// # Examples
///
/// Run the server in the foreground. This will only return if there is an error
/// that causes the server to shut down. This is used to run Merino as a service,
/// such as in production.
///
/// ```no_run
/// # tokio_test::block_on(async {
/// let listener = std::net::TcpListener::bind("127.0.0.1:8080")
///     .expect("Failed to bind port");
/// let settings = merino_settings::Settings::load()
///     .expect("Failed to load settings");
/// let metrics_client = cadence::StatsdClient::from_sink("merino", cadence::NopMetricSink);
/// merino_web::run(listener, metrics_client, settings)
///     .expect("Failed to start server")
///     .await
///     .expect("Fatal error while running server");
/// # })
/// ```
///
/// Run the server as a background task. This will return immediately and process
/// requests. This is useful for tests.
///
/// ```no_run
/// use std::net::TcpListener;
/// use merino_settings::Settings;
///
/// let listener = TcpListener::bind("127.0.0.1:8080")
///     .expect("Failed to bind port");
/// let settings = merino_settings::Settings::load()
///     .expect("Failed to load settings");
/// let metrics_client = cadence::StatsdClient::from_sink("merino", cadence::NopMetricSink);
/// let server = merino_web::run(listener, metrics_client, settings)
///     .expect("Failed to start server");
///
/// /// The server can be stopped with `join_handle::abort()`, if needed.
/// let join_handle = tokio::spawn(server);
/// ```
pub fn run(
    listener: TcpListener,
    metrics_client: StatsdClient,
    settings: Settings,
) -> Result<Server, anyhow::Error> {
    let num_workers = settings.http.workers;

    let moz_log = MozLog::default();

    let location_config = Data::new({
        let mut config =
            actix_web_location::LocationConfig::default().with_metrics(metrics_client.clone());

        if let Some(ref mmdb) = settings.location.maxmind_database {
            config = config.with_provider(
                actix_web_location::providers::MaxMindProvider::from_path(mmdb).context(
                    format!(
                        "Could not set up maxmind location provider with database at {}",
                        mmdb.to_string_lossy(),
                    ),
                )?,
            );
        }

        config.with_provider(FallbackProvider::new(Location::build()))
    });

    let mut server = HttpServer::new(move || {
        App::new()
            // App state
            .app_data(Data::new((&settings).clone()))
            .app_data(location_config.clone())
            .app_data(Data::new(metrics_client.clone()))
            // Middlewares
            .wrap(moz_log.clone())
            .wrap(middleware::Metrics)
            .wrap(middleware::Sentry)
            .wrap(Cors::permissive())
            // The core functionality of Merino
            .service(web::scope("api/v1/suggest").configure(suggest::configure))
            // Add some debugging views
            .service(web::scope("debug").configure(debug::configure))
            .service(root_info)
            // Add the behavior necessary to satisfy Dockerflow.
            .service(web::scope("").configure(dockerflow::configure))
    })
    .listen(listener)?;

    if let Some(n) = num_workers {
        server = server.workers(n);
    }

    let server = server.run();
    Ok(server)
}

/// The root view, to provide information about what this service is.
///
/// This is intended to be seen by people trying to investigate what this service
/// is. It should redirect to documentation, if it is available, or provide a
/// short message otherwise.
#[get("/")]
pub fn root_info(settings: Data<Settings>) -> HttpResponse {
    match &settings.public_documentation {
        Some(redirect_url) => HttpResponse::Found()
            .insert_header(("location", redirect_url.to_string()))
            .finish(),
        None => HttpResponse::Ok().content_type("text/plain").body(
            "Merino is a Mozilla service providing information to the Firefox Suggest feature.",
        ),
    }
}
