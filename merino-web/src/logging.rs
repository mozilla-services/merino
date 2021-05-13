//! Loggers for the request/response cycle.

use actix_web::{dev::ServiceResponse, http::StatusCode};
use tracing::Span;
use tracing_actix_web::{RequestId, RootSpanBuilder};

/// A root span builder for tracing_actix_web to customize the extra fields we
/// log with requests, and to log an event when requests end.
pub struct MerinoRootSpanBuilder;

impl RootSpanBuilder for MerinoRootSpanBuilder {
    fn on_request_start(request: &actix_web::dev::ServiceRequest) -> tracing::Span {
        let http_route: std::borrow::Cow<'static, str> = request
            .match_pattern()
            .map(Into::into)
            .unwrap_or_else(|| "default".into());
        let http_method = request.method().as_str();

        use actix_web::HttpMessage;

        let request_id = request.extensions().get::<RequestId>().cloned().unwrap();

        let span = tracing::info_span!(
            "HTTP request",
            http.method = %http_method,
            http.route = %http_route,
            http.target = %request.uri().path_and_query().map(|p| p.as_str()).unwrap_or(""),
            http.status_code = tracing::field::Empty,
            request_id = %request_id,
            exception.message = tracing::field::Empty,
            exception.details = tracing::field::Empty,
        );

        span
    }

    fn on_request_end<B>(span: Span, outcome: &Result<ServiceResponse<B>, actix_web::Error>) {
        let status = match &outcome {
            Ok(response) => {
                if let Some(error) = response.response().error() {
                    handle_error(span, error)
                } else {
                    span.record("http.status_code", &response.response().status().as_u16());
                    response.status()
                }
            }
            Err(error) => handle_error(span, error),
        };

        match status.as_u16() {
            status_code if (100..400).contains(&status_code) => tracing::info!("Request success"),
            status_code if (400..500).contains(&status_code) => {
                tracing::warn!("Request client error")
            }
            status_code if (500..600).contains(&status_code) => {
                tracing::error!("Request server error")
            }
            status_code => {
                tracing::error!(%status_code, "Request ended with unknown status code {}", status_code);
            }
        };
    }
}

/// Annotate the root request span with information about a request error.
fn handle_error(span: Span, error: &actix_web::Error) -> StatusCode {
    let response_error = error.as_response_error();
    let status = response_error.status_code();
    span.record(
        "exception.message",
        &tracing::field::display(response_error),
    );
    span.record("exception.details", &tracing::field::debug(response_error));
    status
}
