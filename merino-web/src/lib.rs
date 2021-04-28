#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! Web server for [Merino](../merino/index.html)'s public API.

mod debug;
mod dockerflow;
mod errors;
mod suggest;

use std::net::TcpListener;

use actix_cors::Cors;
use actix_web::{dev::Server, web, App, HttpServer};
use merino_settings::Settings;

/// Run the web server
///
/// The returned server is a `Future` that must either be `.await`ed, or run it
/// as a background task using `tokio::spawn`.
///
/// # Examples
///
/// Run the server forever:
///
/// ```
/// use std::net::TcpListener;
///
/// tokio_test::block_on(async {
///     let listener = TcpListener::bind("127.0.0.1:8080")
///         .expect("Failed to bind port");
///     merino_web::run(listener)
///         .expect("Failed to bind address")
///         .await;
/// })
/// ```
///
/// Run the server as a background task:
///
/// ```
/// use std::net::TcpListener;
/// let listener = TcpListener::bind("127.0.0.1:8080")
///     .expect("Failed to bind port");
/// let server = merino_web::run(listener)
///     .expect("Failed to find address");
/// let join_handle = tokio::spawn(server);
/// // The server can be stopped with join_handle::abort();
/// ```
pub fn run(listener: TcpListener, settings: Settings) -> Result<Server, std::io::Error> {
    let num_workers = settings.http.workers;

    let mut server = HttpServer::new(move || {
        App::new()
            .data((&settings).clone())
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
