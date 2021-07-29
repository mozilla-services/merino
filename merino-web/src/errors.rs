//! Any errors that merino-web might generate, and supporting implementations.

use std::collections::HashMap;

use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde_json::Value;
use thiserror::Error;

/// An error that happened in a web handler.
#[derive(Error, Debug)]
pub enum HandlerError {
    /// A generic error, when there is nothing more specific to say.
    #[error("Internal error")]
    Internal,

    /// An error that indicates that one of the request headers is malformed.
    #[error("Malformed header: {0}")]
    MalformedHeader(&'static str),
}

impl ResponseError for HandlerError {
    fn status_code(&self) -> StatusCode {
        match self {
            HandlerError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            HandlerError::MalformedHeader(_) => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse {
        let mut response = HashMap::new();
        response.insert("error".to_owned(), Value::String(format!("{}", self)));
        HttpResponse::InternalServerError().json(response)
    }
}
