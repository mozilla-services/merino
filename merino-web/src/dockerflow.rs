//! An actix-web service to implement [Dockerflow](https://github.com/mozilla-services/Dockerflow).

use std::collections::HashMap;

use actix_web::{get, web, HttpRequest, HttpResponse};
use serde_json::Value;

use crate::errors::HandlerError;

/// Handles required Dockerflow Endpoints.
pub fn service(config: &mut web::ServiceConfig) {
    config
        .service(lbheartbeat)
        .service(heartbeat)
        .service(version)
        .service(test_error);
}

/// Used by the load balancer to indicate that the server can respond to
/// requests. Should just return OK.
#[get("__lbheartbeat__")]
fn lbheartbeat(_: HttpRequest) -> HttpResponse {
    HttpResponse::Ok().body("")
}

/// Return the contents of the `version.json` file created by CircleCI and stored
/// in the Docker root (or the TBD version stored in the Git repo).
#[get("__version__")]
fn version(_: HttpRequest) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("application/json")
        .body(include_str!("../version.json"))
}

/// Returns a status message indicating the current state of the server.
#[get("__heartbeat__")]
fn heartbeat(_: HttpRequest) -> HttpResponse {
    let mut checklist = HashMap::new();
    checklist.insert(
        "version".to_owned(),
        Value::String(env!("CARGO_PKG_VERSION").to_owned()),
    );
    HttpResponse::Ok().json(checklist)
}

/// Returning an API error to test error handling.
#[get("__error__")]
async fn test_error(_: HttpRequest) -> Result<HttpResponse, HandlerError> {
    Err(HandlerError::Internal)
}
