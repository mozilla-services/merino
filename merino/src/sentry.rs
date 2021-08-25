//! Merino integration for Sentry.

use anyhow::Result;
use merino_settings::Settings;

/// Sets up Sentry.
///
/// The returned guard must be held for the duration of the program. Once it is
/// dropped, no more errors will be reported.
pub fn init_sentry(settings: &Settings) -> Result<sentry::ClientInitGuard> {
    let mut config = sentry::apply_defaults(sentry::ClientOptions {
        dsn: settings.sentry.dsn(),
        debug: settings.sentry.debug(),
        release: sentry::release_name!(),
        environment: Some(settings.env.clone().into()),
        ..Default::default()
    });

    if settings.sentry.debug() {
        config = config.add_integration(SentryTracer);
    };

    let guard = sentry::init(config);
    sentry::integrations::panic::PanicIntegration::default();
    Ok(guard)
}

/// Emit tracing::debug events for every Sentry event.
struct SentryTracer;

impl sentry::Integration for SentryTracer {
    fn name(&self) -> &'static str {
        "sentry-tracer"
    }

    fn setup(&self, _options: &mut sentry::ClientOptions) {
        tracing::debug!("setting up SentryTracer");
    }

    fn process_event(
        &self,
        event: sentry::protocol::Event<'static>,
        _options: &sentry::ClientOptions,
    ) -> Option<sentry::protocol::Event<'static>> {
        let exception_descriptions: Vec<_> = event
            .exception
            .values
            .iter()
            .map(|exc| {
                format!(
                    "{}: {}",
                    exc.ty,
                    match &exc.value {
                        Some(value) => value.as_str(),
                        None => "--",
                    }
                )
            })
            .collect();

        tracing::debug!(
            event_id = %event.event_id,
            exceptions = ?exception_descriptions,
            "A sentry error was sent"
        );

        Some(event)
    }
}
