# This Dockerfile uses multi-stage builds to produce very small deployed images
# and optimize usage of layer caching. Docker 17.05 or higher required for
# multi-stage builds.

# Updating this argument will clear the cache of the package installations
# below. This will cause a full rebuild, but it is the only way to get package
# updates with out changing the base image.
ARG APT_CACHE_BUST="2021-05-13"

# =============================================================================
# Pull in the version of cargo-chef we plan to use, so that all the below steps
# use a consistent set of versions.
FROM lukemathwalker/cargo-chef:0.1.33-rust-1.58-buster as chef
WORKDIR /app

# =============================================================================
# Analyze the project, and produce a plan to compile its dependcies. This will
# be run every time. The output should only change if the dependencies of the
# project change, or if significant details of the build process change.
FROM chef as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# =============================================================================
# Use the plan from above to build only the dependencies of the project. This
# should almost always be pulled straight from cache unless dependencies or the
# build process change.
FROM chef as cacher
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json -p merino

# =============================================================================
# Now build the project, taking advantage of the cached dependencies from above.
FROM chef as builder
ARG APT_CACHE_BUST

RUN mkdir -m 755 bin
RUN apt-get -qq update && \
    apt-get -qq upgrade
RUN cargo --version && \
    rustc --version
COPY . .
COPY --from=cacher /app/target target
COPY --from=cacher $CARGO_HOME $CARGO_HOME

RUN cargo build --release -p merino
RUN cp /app/target/release/merino /app/bin

# =============================================================================
# Finally prepare a Docker image based on a slim image that only contains the
# files needed to run the project.
FROM debian:buster-slim as runtime
ARG APT_CACHE_BUST

RUN apt-get -qq update && \
    apt-get -qq upgrade && \
    apt-get -qq install ca-certificates && \
    rm -rf /var/lib/apt/lists
RUN groupadd --gid 10001 app && \
    useradd --uid 10001 --gid 10001 --home /app --create-home app

COPY --from=builder /app/bin /app/bin
COPY --from=builder /app/version.json /app
COPY --from=builder /app/config /app/config

WORKDIR /app
USER app

CMD ["/app/bin/merino"]
