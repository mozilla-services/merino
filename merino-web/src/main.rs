#![warn(missing_docs, clippy::missing_docs_in_private_items)]

//! Web server for [Merino](../merino/index.html)'s public API.

mod dockerflow;
mod errors;
mod suggest;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer};

/// Run the server.
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || App::new().wrap(Cors::permissive()).configure(configure_app))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}

/// Create a `actix_web::App` configured with all routes. Does not include middleware.
pub fn configure_app(cfg: &mut web::ServiceConfig) {
    cfg
        // The core functionality of Merino
        .service(web::scope("/api/v1/suggest").configure(suggest::service))
        // Add the behavior necessary to satisfy Dockerflow
        .service(web::scope("/").configure(dockerflow::service));
}
