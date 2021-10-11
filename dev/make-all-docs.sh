#!/usr/bin/env bash
set -eu

# Remove any previous built docs
mdbook clean

# Build book-docs
mdbook build

# Build rust-docs
cargo doc --document-private-items --workspace

# Copy the rust-docs into the book
mkdir -p book/rustdoc
cp -r target/doc/* book/rustdoc

# Add a redirect for the old location of the docs
# Merino crate's docs.
mkdir -p book/merino/merino
echo '<meta http-equiv="refresh" content="0; URL=../index.html" />' > book/merino/index.html

# Rustdoc doesn't include a base index.html. Add one that redirects to the base
# Merino crate's docs.
echo '<meta http-equiv="refresh" content="0; URL=merino/" />' > book/rustdoc/index.html
