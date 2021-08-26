# This Dockerfile uses multi-stage builds to produce very small deployed images
# and optimize usage of layer caching. Docker 17.05 or higher required for
# multi-stage builds.

# Updating this argument will clear the cache of the package installations
# below. This will cause a full rebuild, but it is the only way to get package
# updates with out changing the base image.
ARG CACHE_BUST="2021-05-13"

# =============================================================================
# Analyze the project, and produce a plan to compile its dependcies. This will
# be run every time. The output should only change if the dependencies of the
# project change, or if significant details of the build process change.

# Note that this is not actually an alpha version. The image is mistagged. See
# https://github.com/LukeMathWalker/cargo-chef/issues/86
FROM lukemathwalker/cargo-chef:0.1.26-alpha.0-rust-1.54-buster as planner
WORKDIR /app
COPY . .
RUN cargo version | tee cargo-version.txt
RUN cargo chef prepare --recipe-path recipe.json

# =============================================================================
# Use the plan from above to build only the dependencies of the project. This
# should almost always be pulled straight from cache unless dependencies or the
# build process change.

# Note that this is not actually an alpha version. The image is mistagged. See
# https://github.com/LukeMathWalker/cargo-chef/issues/86
FROM lukemathwalker/cargo-chef:0.1.26-alpha.0-rust-1.54-buster as cacher
WORKDIR /app

COPY --from=planner /app/cargo-version.txt cargo-version.txt
COPY ./.circleci/check-cargo-version.sh .
RUN ./check-cargo-version.sh cargo-version.txt

COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# =============================================================================
# Now build the project, taking advantage of the cached dependencies from
# above. The version number here should be the same as the version used by cargo-chef
FROM rust:1.54 as builder
WORKDIR /app
ARG RUST_TOOLCHAIN=stable
ARG CACHE_BUST

COPY --from=planner /app/cargo-version.txt cargo-version.txt
COPY ./.circleci/check-cargo-version.sh .
RUN ./check-cargo-version.sh cargo-version.txt

RUN mkdir -m 755 bin
RUN apt-get -qq update && \
    apt-get -qq upgrade
RUN rustup default ${RUST_TOOLCHAIN} && \
    cargo --version && \
    rustc --version
COPY . .
COPY --from=cacher /app/target target
COPY --from=cacher $CARGO_HOME $CARGO_HOME

RUN cargo build --release --bin merino
RUN cp /app/target/release/merino /app/bin

# =============================================================================
# Finally prepare a Docker image based on a slim image that only contains the
# files needed to run the project.
FROM debian:buster-slim as runtime
ARG CACHE_BUST

RUN apt-get -qq update && \
    apt-get -qq upgrade && \
    apt-get -qq install ca-certificates && \
    rm -rf /var/lib/apt/lists
RUN groupadd --gid 10001 app && \
    useradd --uid 10001 --gid 10001 --home /app --create-home app

COPY --from=builder /app/bin /app/bin
COPY --from=builder /app/version.json /app
COPY --from=builder /app/config /app/config

ARG HOST=0.0.0.0
ARG PORT=8080
ENV MERINO_HTTP__LISTEN="${HOST}:${PORT}"

WORKDIR /app
USER app
EXPOSE ${PORT}

CMD ["/app/bin/merino"]
