# Logging and Metrics

To get data out of Merino and into observable systems, we use _metrics_ and
_logging_. Each has a unique use case. Note that in general, because of the scale
we work at, adding a metric or log event in production is not free, and if we
are careless can end up costing quite a bit. Record what is needed, but don't go
over board.

All data collection that happens in production (logging at INFO, WARN, or ERROR
levels; and metrics) should be documented in [`docs/data.md`](../data.md).

## Logging

Merino uses [Tracing][] for logging, which "is a framework for instrumenting
Rust programs to collect structured, event-based diagnostic information". Below
are some notes about using Tracing in Merino, but consider reading their docs
for more information.

[tracing]: https://crates.io/crates/tracing

The basic way to interact with tracing is via the macros `tracing::error!`,
`tracing::warn!`, `tracing::info!`, `tracing::debug!`, and `tracing::trace!`.

Tracing can output logs in various formats, including a JSON format for
production. In these docs we'll use a pretty, human readable format that spreads
logs over multiple lines to include more information in a readable way.

### Types

MozLog requires that all messages have a `type` value. If one is not provided,
our logging systems use `"<unknown>"` as a type value. All `INFO`, `WARN`, and
`ERROR` messages should have a type field, specified like:

```rust
tracing::warn!(
  r#type = "suggest.providers.multi.created-empty",
  id = %provider.id,
  "An empty MultiProvider was created"
);
```

In general, the log _message_ ("An empty MultiProvider was created") and the log
_type_ should both tell the reader what has happened. The difference is that the
message is for humans and the type is for machines.

Type should be a dotted path to the file you're working in, with any `merino-`
prefix removed, ending in a code specific to the error. This does not strictly
need to follow the file system hierarchy, and stability over time is more
important than refactoring.

### Levels

Tracing provides five log levels that should be familiar. This is what we mean
by them in Merino:

- `ERROR` - There was a problem, and the task was not completable. This usually
  results in a 500 being sent to the user. All error logs encountered in
  production are reported to Sentry and should be considered a bug. If it isn't
  a bug, it shouldn't be logged as an error.

- `WARNING` - There was a problem, but the task was able to recover. This
  doesn't usually affect what the user sees. Warnings are suitable for
  unexpected but "in-spec" issues, like a sync job not returning an empty set or
  using a deprecated function. These are not reported to Sentry.

- `INFO` - This is the default level of the production service. Use for logging
  that something happened that isn't a problem. and we care about in production.
  This is the level that Merino uses for it's one-per-request logs and sync
  status messages. Be careful adding new per-request logs at this level, as they
  can be expensive.

- `DEBUG` - This is the default level for developers running code locally. Use
  this to give insight into how the system is working, but keep in mind that
  this will be on by default, so don't be too noisy. Generally this should
  summarize what's happening, but not give the small details like a log line for
  every iteration of a loop. Since this is off in production, there are no cost
  concerns.

- `TRACE` - This level is hidden by default in all environments, including
  tests. Add this for very detailed logs of what specific functions or objects
  are doing. To see these logs, you'll need to turn up the logging level for the
  area of the code you're in. See [the logging settings](../ops.md#logging) for
  more details. If you add logs to figure out why something isn't working or why
  a test isn't passing, do so at the `TRACE` level, and consider leaving them in
  the code for future debuggers.

### Including data

If you want to log something that includes the contents of a variable, in other
libraries you might use string interpolation like
`tracing::error!("could not find file: {}", file_path)`. This works in Tracing,
but there is a better way:

```rust
tracing::error!(?file_path, r#type = "file_handler.missing", "could not find file");
```

This would produce a log event like

```log
Oct 27 15:51:35.134 ERROR merino: could not find file, file_path: "an/important/path.txt", type: "file_handler.missing"
  at merino/src/file_handler.rs:65
  in merino::file_handler::load
```

By including the `file_path` before the log line, it is included as structured
data. This will be machine-readable and can be used for better parsing down the
line. In general, you should prefer structured logging for including data in log
events.

## Metrics

Metrics are handled by [Cadence][] in [Statsd][] format
https://www.datadoghq.com/blog/statsd/.

[cadence]: https://crates.io/crates/cadence

Unlike logging, the primary way that metrics reporting can cost a lot is in
_cardinality_. The number of metric IDs we have and the combination of tag
values that we supply. Often the number of individual events doesn't matter as
much, since multiple events are aggregated together.
