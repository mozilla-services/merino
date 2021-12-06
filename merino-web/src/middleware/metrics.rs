//! Middlewares for reporting Metrics in Merino.

use crate::errors::HandlerError;
use actix_web::{
    dev::{Service, ServiceRequest, ServiceResponse, Transform},
    Error as ActixError,
};
use cadence::{StatsdClient, Timed};
use std::{
    fmt,
    future::{ready, Future, Ready},
    pin::Pin,
    task::Context,
    time::Instant,
};

/// Factory for [`MetricsMiddleware`].
pub struct Metrics;

/// Middleware to record request metrics.
pub struct MetricsMiddleware<S> {
    /// The wrapped service.
    service: S,
}

impl<S> Transform<S, ServiceRequest> for Metrics
where
    S: Service<ServiceRequest, Response = ServiceResponse>,
    S::Future: 'static,
    S::Error: fmt::Debug,
{
    type Response = ServiceResponse;

    type Error = ActixError;

    type Transform = MetricsMiddleware<S>;

    type InitError = ();

    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(MetricsMiddleware { service }))
    }
}

impl<S> Service<ServiceRequest> for MetricsMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse>,
    S::Future: 'static,
    S::Error: fmt::Debug,
{
    type Response = ServiceResponse;

    type Error = ActixError;

    #[allow(clippy::type_complexity)]
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, ctx: &mut Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx).map_err(|error| {
            tracing::error!(
                r#type = "web.metrics.polling-error",
                ?error,
                "Error polling service from metrics middleware"
            );
            HandlerError::internal().into()
        })
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let start = Instant::now();
        let path = req.path().to_string();
        let metrics_client = req.app_data::<StatsdClient>().cloned();
        let fut = self.service.call(req);

        Box::pin(async move {
            let response = fut.await.map_err(|_err| HandlerError::internal())?;
            if let Some(metrics_client) = metrics_client {
                let lapsed = Instant::now().duration_since(start);
                metrics_client
                    .time_with_tags("request.duration", lapsed)
                    .with_tag("path", &path)
                    .send();
            } else if cfg!(debug) {
                panic!("No metrics client configured, but metrics middleware attached");
            }
            Ok(response)
        })
    }
}
