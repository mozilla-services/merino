//! # High level overview of Merino
//!
//! This project is structured as a [Cargo Workspace][] that contains one crate
//! for each broad area of behavior for Merino. This structure is advantageous
//! because the crates can be handled either individually or as a group. When
//! compiling, each crate can be compiled in parallel, where dependencies allow,
//! and when running tests, each test suite can be run separately or together.
//! This also provides an advantage if we choose to re-use any of these crates in
//! other projects, or if we publish the crates to Crates.io.
//!
//! [Cargo Workspace]: https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html
//!
//! This is a brief overview of the crates found in the repository. For more
//! details, see the specific crate docs.
//!
//! ## [`merino`](../)
//!
//! This is the main Merino application, and one of the *binary* crates in the
//! repository. It brings together and configures the other crates to create a
//! production-like environment for Firefox Suggest.
//!
//! ## [`merino-settings`](../../merino_settings/index.html)
//!
//! This defines and documents the settings of the application. These settings
//! should be initialized by one of the *binary* crates, and passed into the
//! other crates to configure them.
//!
//! ## [`merino-web`](../../merino_web/index.html)
//!
//! This crate provides an HTTP API to access Merino, including providing
//! observability into the running of the application via that API.
//!
//! ## [`merino-suggest`](../../merino_suggest/index.html)
//!
//! This is a *domain* crate that defines the data model and traits needed to
//! provide suggestions to Firefox.
//!
//! ## [`merino-adm`](../../merino_adm/index.html)
//!
//! This crate provides integration with the AdMarketplace APIs, and implements
//! the traits from `merino-suggest`.
//!
//! ## [`merino-integration-tests`](../../merino_integration_tests/index.html)
//!
//! This crate is a separate test system. It works much like `merino`, in that it
//! brings together the other crates to produce a complete Merino environment.
//! However, this binary crate produces an application that exercise the service
//! as a whole, instead of providing a server to manual test against.
