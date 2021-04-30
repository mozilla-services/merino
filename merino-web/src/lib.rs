#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! Web server for [Merino](../merino/index.html)'s public API.

mod debug;
mod dockerflow;
mod errors;
mod logging;
mod suggest;

use actix_cors::Cors;
use actix_web::{dev::Server, web, App, HttpServer};
use logging::MerinoRootSpanBuilder;
use merino_settings::Settings;
use std::net::TcpListener;

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
/// merino_web::run(listener, settings)
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
/// let server = merino_web::run(listener, settings)
///     .expect("Failed to start server");
///
/// /// The server can be stopped with `join_handle::abort()`, if needed.
/// let join_handle = tokio::spawn(server);
/// ```
pub fn run(listener: TcpListener, settings: Settings) -> Result<Server, std::io::Error> {
    let num_workers = settings.http.workers;

    let mut server = HttpServer::new(move || {
        App::new()
            .data::<Settings>((&settings).clone())
            .wrap(tracing_actix_web::TracingLogger::<MerinoRootSpanBuilder>::new())
            .wrap(Cors::permissive())
            // The core functionality of Merino
            .service(web::scope("api/v1/suggest").configure(suggest::configure))
            // Add some debugging views
            .service(web::scope("debug").configure(debug::configure))
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
