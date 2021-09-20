//! An actix-web service to introspect Merino if the `debug` setting is enabled.
//! The handlers here should all verify that debug is enabled.

use actix_web::{
    get,
    web::{self, Data},
    HttpResponse,
};
use merino_settings::Settings;

/// Handles required Dockerflow Endpoints.
pub fn configure(config: &mut web::ServiceConfig) {
    config.service(settings);
}

/// In debug mode, show the settings of the app.
#[get("settings")]
async fn settings(settings: Data<Settings>) -> HttpResponse {
    if settings.debug {
        HttpResponse::Ok().json(settings)
    } else {
        HttpResponse::NotFound().body("")
    }
}
