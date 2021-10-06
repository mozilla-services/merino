//! Middlewares for using Sentry in Merino.

use crate::errors::HandlerError;
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    error::Error as ActixError,
};
use futures_util::future::LocalBoxFuture;
use sentry::protocol::Event;
use std::{
    error::Error as StdError,
    fmt,
    future::{ready, Ready},
    task::{Context, Poll},
};

/// Wrapper for Sentry error reporting, since no version of sentry-actix is
/// compatible with our stack. Old versions don't have modern Actix middlewares,
/// and new versions require servers newer than ours.
#[derive(Debug, Default)]
pub struct Sentry;

impl<S> Transform<S, ServiceRequest> for Sentry
where
    S: Service<ServiceRequest, Response = ServiceResponse> + 'static,
    S::Future: 'static,
    S::Error: fmt::Debug,
{
    type Response = ServiceResponse;
    type Error = ActixError;
    type InitError = ();
    type Transform = SentryMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(SentryMiddleware { service }))
    }
}

/// Middleware to catch events from request handlers and send them to Sentry.
#[derive(Debug)]
pub struct SentryMiddleware<S> {
    /// The wrapped service
    service: S,
}

impl<S> SentryMiddleware<S> {
    /// Custom `sentry::event_from_error` for `HandlerError`
    ///
    /// `sentry::event_from_error` can't access `std::Error` backtraces as its
    /// `backtrace()` method is currently Rust nightly only. This function works
    /// against `HandlerError` instead to access its backtrace.
    pub fn event_from_error(err: &HandlerError) -> Event<'static> {
        let mut exceptions = vec![Self::exception_from_error_with_backtrace(err)];

        let mut source = err.source();
        while let Some(err) = source {
            let exception = if let Some(err) = err.downcast_ref() {
                Self::exception_from_error_with_backtrace(err)
            } else {
                Self::exception_from_error(err)
            };
            exceptions.push(exception);
            source = err.source();
        }

        exceptions.reverse();
        Event {
            exception: exceptions.into(),
            level: sentry::protocol::Level::Error,
            ..Default::default()
        }
    }

    /// Custom `exception_from_error` support function for `HandlerError`
    ///
    /// Based moreso on sentry_failure's `exception_from_single_fail`.
    fn exception_from_error_with_backtrace(err: &HandlerError) -> sentry::protocol::Exception {
        let mut exception = Self::exception_from_error(err);
        // format the stack trace with alternate debug to get addresses
        let bt = format!("{:#?}", err.backtrace);
        exception.stacktrace = sentry_backtrace::parse_stacktrace(&bt);
        exception
    }

    /// Exact copy of sentry's unfortunately private `exception_from_error`
    fn exception_from_error<E: StdError + ?Sized>(err: &E) -> sentry::protocol::Exception {
        let dbg = format!("{:?}", err);
        sentry::protocol::Exception {
            ty: sentry::parse_type_from_debug(&dbg).to_owned(),
            value: Some(err.to_string()),
            ..Default::default()
        }
    }
}

impl<S> Service<ServiceRequest> for SentryMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse>,
    S::Future: 'static,
    S::Error: fmt::Debug,
{
    type Response = ServiceResponse;
    type Error = ActixError;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx).map_err(|error| {
            tracing::error!(?error, "Error polling service");
            HandlerError::internal().into()
        })
    }

    #[tracing::instrument(level = "DEBUG", skip(self, req))]
    fn call(&self, req: ServiceRequest) -> Self::Future {
        // sentry
        let hub = sentry::Hub::current();
        let transaction = if let Some(name) = req.match_name() {
            Some(String::from(name))
        } else {
            req.match_pattern()
        };
        hub.configure_scope(|scope| {
            scope.set_transaction(transaction.as_deref());
        });

        let fut = self.service.call(req);

        Box::pin(async move {
            let response = fut.await.map_err(|error| {
                tracing::error!(?error, "handler error");
                HandlerError::internal()
            })?;
            tracing::trace!(?response, "checking response for errors");

            match response.response().error() {
                None => (),
                Some(error) => {
                    tracing::trace!(?error, "Found error on response");
                    if let Some(handler_error) = error.as_error::<HandlerError>() {
                        hub.capture_event(Self::event_from_error(handler_error));
                    }
                }
            }

            Ok(response)
        })
    }
}
