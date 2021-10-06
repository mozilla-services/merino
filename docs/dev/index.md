# Developer documentation for working on Merino

## tl;dr

Here are some useful commands when working on Merino.

Run the main app

```shell
$ docker-compose -f dev/docker-compose.yaml -d
$ cargo run -p merino
```

Run tests

```shell
$ docker-compose -f dev/docker-compose.yaml -d
$ cargo test
```

## Documentation

You can generate documentation, both code level and book level, for Merino and
all related crates by running `./dev/make-all-docs.sh`.

Pre-built code docs are available online at
<https://mozilla.github.io/merino/rustdoc/>.

## Local configuration

The default configuration of Merino is `development`, which has human-oriented
logging and debugging enabled. For settings that you wish to change in the
development configuration, you have two options, listed below.

> For full details, make sure to check out the documentation for [Merino's
> setting system][../settings.md]

### Update the defaults

If the change you want to make makes the system better for most development
tasks, consider adding it to `config/development.yaml`, so that other developers
can take advantage of it. You can look at `config/base.yaml`, which defines all
requires configuration, to see an example of the structure.

It is not suitable to put secrets in `config/development.yaml`.

### Create a local override

For local changes to adapt to your machine or tastes, you can put the
configuration in `config/local.yaml`. These file doesn't exist by default. These
changes won't be a part of the git history, so it is safe to put secrets here,
if needed. Importantly, it should never be _required_ to have a `local.yaml` to
run Merino in a development setting.

## Repository structure

This project is structured as a [Cargo Workspace][] that contains one crate for
each broad area of behavior for Merino. This structure is advantageous because
the crates can be handled either individually or as a group. When compiling,
each crate can be compiled in parallel, where dependencies allow, and when
running tests, each test suite can be run separately or together. This also
provides an advantage if we choose to re-use any of these crates in other
projects, or if we publish the crates to Crates.io.

[cargo workspace]: https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html

## Project crates

This is a brief overview of the crates found in the repository. For more
details, see the specific crate docs.

### [`merino`](../rustdoc/merino/)

This is the main Merino application, and one of the _binary_ crates in the
repository. It brings together and configures the other crates to create a
production-like environment for Firefox Suggest.

### [`merino-settings`](../rustdoc/merino_settings/)

This defines and documents the settings of the application. These settings
should be initialized by one of the _binary_ crates, and passed into the other
crates to configure them.

### [`merino-web`](../rustdoc/merino_web/)

This crate provides an HTTP API to access Merino, including providing
observability into the running of the application via that API.

### [`merino-suggest`](../rustdoc/merino_suggest/)

This is a _domain_ crate that defines the data model and traits needed to
provide suggestions to Firefox.

### [`merino-cache`](../rustdoc/merino_cache/)

This crate contains domain models and behavior for Merino's caching
functionality.

### [`merino-adm`](../rustdoc/merino_adm/)

This crate provides integration with the AdMarketplace APIs, and implements the
traits from `merino-suggest`.

### [`merino-showroom`](./showroom.html)

This is not a Rust crate, but instead a small Javascript application. It can be
used to test Merino during development and demos.

### [`merino-integration-tests`](../rustdoc/merino_integration_tests/)

This crate is a separate test system. It works much like `merino`, in that it
brings together the other crates to produce a complete Merino environment.
However, this binary crate produces an application that exercise the service as
a whole, instead of providing a server to manual test against.

### [`merino-integration-tests-macro`](../rustdoc/merino_integration_tests_macro/)

This crate provides a procmacro used in `merino-integration-tests`. Rust
requires that procmacros be in their own crate.

## Recommended Tools

- [rust-analyzer][] - IDE-like tools for many editors. This provides easy access
  to type inference and documentation while editing Rust code, which can make
  the development process much easier.
- [cargo-watch][] - A Cargo subcommand that re-runs a task when files change.
  Very useful for things like `cargo watch -x clippy` or
  `cargo watch -x "test -- merino-adm"`.

[rust-analyzer]: https://rust-analyzer.github.io/
[cargo-watch]: https://crates.io/crates/cargo-watch

## Recommended Reading

These works have influenced the design of Merino.

- The Contextual Services
  [Skeleton Actix project](https://github.com/mozilla-services/skeleton/)
- [Zero to Production in Rust](https://www.zero2prod.com/) by Luca Palmieri
- [Error Handling Isn't All About Errors](https://www.youtube.com/watch?v=rAF8mLI0naQ),
  by Jane "[yaahc](https://twitter.com/yaahc_/)" Lusby, from RustConf 2020.

Run dependency servers

```shell
$ cd dev
$ docker-compose up
```
