//! Any errors that merino-web might generate, and supporting implementations.

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use backtrace::Backtrace;
use serde_json::Value;
use thiserror::Error;

/// The Standard Error for most of Merino
#[derive(Debug)]
pub struct HandlerError {
    kind: HandlerErrorKind,
    pub(crate) backtrace: Backtrace,
}

/// An error that happened in a web handler.
#[derive(Error, Debug)]
pub enum HandlerErrorKind {
    /// A generic error, when there is nothing more specific to say.
    #[error("Internal error")]
    Internal,

    /// An error that indicates that one of the request headers is malformed.
    #[error("Malformed header: {0}")]
    MalformedHeader(&'static str),
}

impl HandlerErrorKind {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            Self::MalformedHeader(_) => StatusCode::BAD_REQUEST,
        }
    }

    pub fn error_response(&self) -> HttpResponse {
        let mut response = HashMap::new();
        response.insert("error".to_owned(), Value::String(format!("{}", self)));
        HttpResponse::InternalServerError().json(response)
    }
}

impl From<HandlerErrorKind> for actix_web::Error {
    fn from(kind: HandlerErrorKind) -> Self {
        let error: HandlerError = kind.into();
        error.into()
    }
}

impl HandlerError {
    pub fn kind(&self) -> &HandlerErrorKind {
        &self.kind
    }

    pub fn internal() -> Self {
        HandlerErrorKind::Internal.into()
    }
}

impl Error for HandlerError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.kind.source()
    }
}

impl<T> From<T> for HandlerError
where
    HandlerErrorKind: From<T>,
{
    fn from(item: T) -> Self {
        HandlerError {
            kind: HandlerErrorKind::from(item),
            backtrace: Backtrace::new(),
        }
    }
}

impl fmt::Display for HandlerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(f)
    }
}

impl ResponseError for HandlerError {
    fn status_code(&self) -> StatusCode {
        self.kind().status_code()
    }

    fn error_response(&self) -> HttpResponse {
        let mut response = HashMap::new();
        response.insert(
            "error".to_owned(),
            Value::String(format!("{}", self.kind())),
        );
        HttpResponse::InternalServerError().json(response)
    }
}
