//! # Developer documentation for working on Merino
//!
//! Currently Merino has no external dependencies, so it is simple to set up.
//!
//! ```shell
//! git clone https://github.com/mozilla-services/merino
//! cd merino
//! cargo run
//! ```
//!
//! Tests likewise are run with the standard `cargo test` command.
//!
//! ## Local configuration
//!
//! The default configuration of Merino is development, which has
//! human-oriented logging and debugging enabled. For settings that you wish to
//! change in the development configuration, you have three options.
//!
//! > For full details, make sure to check out the documentation for
//! > [`merino_settings`].
//!
//! ### Update the defaults
//!
//! If the change you want to make definitely makes the system better for most
//! development tasks, consider adding it to `config/development.yaml`, so that
//! your fellow developers can take advantage of it. You can look at
//! `config/base.yaml`, which defines all requires configuration, to see an
//! example of the structure.
//!
//! It is not suitable to put secrets in `config/development.yaml`.
//!
//! ### Create a local override
//!
//! For local changes to adapt to your machine or tastes, you can put the
//! configuration in `config/local.yaml`. These file doesn't exist by default.
//! These changes won't be a part of the git history, so it is safe to put
//! secrets here, if needed.
//!
//! ### Create a new configuration
//!
//! For changes that aren't suitable for the default developer configuration, but
//! may be useful for others to reference, or that you want to use across
//! multiple development computers, consider creating a new configuration.
//!
//! You can create a file `config/<yourname>.yaml`, and add that file to the
//! repository. This will be based on the *base* configuration, not the
//! development one, so it is likely a good idea to start by copying the
//! `config/development.yaml` file.
//!
//! Then you'll need to configure Merino to use that environment. Set the
//! environment variable `MERINO_ENV=<yourname>`. There are many ways to set this
//! environment variable, which are out of scope for this document.
//!
//! Since this file is meant to be checked into the repository, it is **not**
//! suitable for secrets.
//!
//! > Note that none of the above overrides are uses in tests. If you need to
//! > configure the test environment, you can edit `config/test.yaml` or create
//! > `config/local_test.yaml`.
//!
//! ## Recommended Tools
//!
//! * [rust-analyzer][] - IDE-like tools for many editors. This provides easy
//!   access to type inference and documentation while editing Rust code, which can
//!   make the development process much easier.
//! * [cargo-watch][] - A Cargo subcommand that re-runs a task when files change.
//!   Very useful for things like `cargo watch -x clippy` or `cargo watch -x "test
//!   -- merino-adm"`.
//!
//! [rust-analyzer]: https://rust-analyzer.github.io/
//! [cargo-watch]: https://crates.io/crates/cargo-watch
//!
//! ## Recommended Reading
//!
//! These works have influenced the design of Merino.
//!
//! * The Contextual Services [Skeleton Actix
//!   project](https://github.com/mozilla-services/skeleton/)
//! * [Zero to Production in Rust](https://www.zero2prod.com/) by Luca Palmieri
