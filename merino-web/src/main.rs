mod dockerflow;
mod errors;
mod suggest;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new().wrap(Cors::permissive()).configure(configure_app)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

pub fn configure_app(cfg: &mut web::ServiceConfig) {
    cfg
        // The core functionality of Merino
        .service(web::scope("/api/v1/suggest").configure(suggest::service))
        // Add the behavior necessary to satisfy Dockerflow
        .service(web::scope("/").configure(dockerflow::service));
}
