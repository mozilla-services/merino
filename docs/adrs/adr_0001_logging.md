# Choosing a logging library for Merino

- Status: accepted
- Date: 2021-05-11

Tracking issue:
[mozilla-services/merino#15](https://github.com/mozilla-services/merino/issues/15)

## Context and Problem Statement

Merino needs a system to produce logging. Rust has several options to do this,
with no clear winner. What library should Merino use?

### The `log` Crate

There is a de facto standard logging library for Rust,
[`log`](https://crates.io/crates/log). It is an important part of the decision,
but it is not a solution to the problem: it is a logging _facade_ that provides
a way for libraries to produce their own logs, but does not provide any logging
capabilities itself. Without a logger crate to use it with, the logs compile
away to nothing.

## Decision Drivers

- Sentry integration should be available
- MozLog format should be supported
- There should be both human and machine friendly logging output options.
- Library compatibility

## Considered Options

1. [slog](https://crates.io/crates/slog)
2. [tracing](https://crates.io/crates/tracing)

- Something from the `log` ecosystem

## Decision Outcome

Option 2 - Tracing. Although Slog has more momentum at this time, Tracing sets
us up for a better set of tools long term.

## Pros and Cons of the Options <!-- optional -->

### Option 1 - Slog

> `slog` is an ecosystem of reusable components for structured, extensible,
> composable and contextual logging for Rust.

Slog's first release was 0.1.0 in June of 2016, and it's 1.0 release was in
September of 2016.

To use it, developers create logger objects that provide methods to produce log
lines and create sub-loggers. These loggers must be passed to lower level code
explicitly to be used.

Slog is compatible with the de facto `log` crate. Log lines emitted using that
system (such as by libraries) can be routed to a logger object, with no loss of
detail. Few libraries use slog directly, and none of the libraries that are
currently used in Merino do at all. Since libraries aren't using `slog`, it will
be harder to make them participate in the logging hierarchy.

Structured logging is supported. Logs can carry associated data, which can aid
in the machine readability of logs.

The tree of loggers is built explicitly through the loggers objects, and
subloggers must be aware of superloggers, since the only way to get a sublogger
is to call a method on a logger. This separates the logging structure from the
call stack, but makes it awkward to recover that information should it be
helpful. This means that logging generally has to be passed as arguments to many
functions, making the tree of loggers less flexible.

The Sentry library for Rust has support for slog. There is a a
[MozLog crate](https://crates.io/crates/slog-mozlog-json) for slog that Mozilla
wrote.

- Good, because structured logging helps provide more useful logs.
- Good, because it has Sentry integration.
- Good, because it already has MozLog integration.
- Bad, because it has little library support.
- Bad, because explicitly building the logging tree is rigid.

### Option 2 - Tracing

> `tracing` is a framework for instrumenting Rust programs to collect
> structured, event-based diagnostic information.

Tracing's first release was 0.0.0 in November of 2017. It has not had a 1.0
release. It's latest release is 0.1.26, published April 2021.

To use it, developers set up a global or scope-based subscriber of logs, which
collects _spans_ and _events_ that are generated in the code. Spans can be
`enter`ed and `exit`ed. During the time between these, all spans and events are
associated with the entered span as their parent. This association happens
explicitly, and can cross call boundaries. However, spans can be entered and
exited in more fine grained ways if needed.

Tracing is compatible with the de facto `log` crate. Log lines emitted using
that system are seen as events in Tracing, with no loss of detail.

Structured logging is supported. Both spans and events can carry associated
data. This data can be accessed hierarchically, building up a context of
execution.

The tree of spans is built by entering spans. Loggers do not have to be aware of
their parents, their logs are placed in the context of whatever set of spans has
been entered at that moment. because [ one like we did with slog.

Tracing is developed as a part of the Tokio ecosystem, the same that our web
framework (Actix), http client (Reqwest), and async runtime (Tokio) are
developed under. It has some library support. Additionally, since the tree of
spans is built more implicitly, libraries that use the `log` facade can
participate in our structured logging.

[sentry-tracing]: https://github.com/getsentry/sentry-rust/issues/180

- Good, because structured logging helps provide more useful logs.
- Good, because it has good integration into the libraries we use.
- Good, because implicitly building the logging hierarchy and context is
  flexible.
- Bad, because it lacks Sentry support.
- Bad, because it lacks MozLog support.

### Option 3 - Something in the `log` ecosystem

This option was not considered deeply because `log` does not support structured
logging. Not being able to attach concrete data to logs makes much of the
logging tasks much harder.

- Bad, because it lacks structured logging.
