//! An actix-web service to implement [Dockerflow](https://github.com/mozilla-services/Dockerflow).

use std::collections::HashMap;

use actix_web::{get, web, HttpRequest, HttpResponse};
use serde::Serialize;

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
        .body(include_str!("../../version.json"))
}

/// The status of an individual check, or the whole system, as reported by /__heartbeat__.
#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum CheckStatus {
    /// Everything is OK
    Ok,
    /// The check could not determine the status.
    Unknown,
    /// Something is wrong, but it is not interrupting the system.
    #[allow(dead_code)]
    Warn,
    /// Something is wrong, and it is interrupting the system.
    #[allow(dead_code)]
    Error,
}

impl Default for CheckStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

/// A response to the `/__heartbeat__` endpoint.
#[derive(Debug, Default)]
struct HeartbeatResponse {
    /// Any checks that are relevant to the state of the system.
    checks: HashMap<String, CheckStatus>,
}

impl HeartbeatResponse {
    /// The overall status of all checks.
    ///
    /// Takes the worst state of any check contained, or `CheckStatus::Unknown`
    /// if there are no contained checks.
    fn status(&self) -> CheckStatus {
        self.checks
            .values()
            .copied()
            .max()
            .unwrap_or(CheckStatus::Unknown)
    }

    /// Add the results of a check.
    fn add_check<S: Into<String>>(&mut self, name: S, check: CheckStatus) {
        self.checks.insert(name.into(), check);
    }
}

// Serde doesn't have a concept of "derived" fields for serialization. So
// instead define a concrete type with the calculated field, and delegate
// serialization to that.
impl Serialize for HeartbeatResponse {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        #[allow(clippy::missing_docs_in_private_items)]
        struct Extended<'a> {
            status: CheckStatus,
            checks: &'a HashMap<String, CheckStatus>,
        }

        let ext = Extended {
            status: self.status(),
            checks: &self.checks,
        };

        ext.serialize(serializer)
    }
}

/// Returns a status message indicating the current state of the server.
#[get("__heartbeat__")]
fn heartbeat(_: HttpRequest) -> HttpResponse {
    let mut checklist = HeartbeatResponse::default();
    checklist.add_check("heartbeat", CheckStatus::Ok);
    HttpResponse::Ok().json(checklist)
}

/// Returning an API error to test error handling.
#[get("__error__")]
async fn test_error(_: HttpRequest) -> Result<HttpResponse, HandlerError> {
    Err(HandlerError::Internal)
}
