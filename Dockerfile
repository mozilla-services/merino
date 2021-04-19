# This Dockerfile uses multi-stage builds to produce very small deployed
# images. Docker 17.05 or higher required for multi-stage builds

FROM rust:1.51 as builder
WORKDIR /app
ARG APPNAME=merino
ARG RUST_TOOLCHAIN=stable
RUN apt-get -qq update && \
    apt-get -qq upgrade
RUN rustup default ${RUST_TOOLCHAIN} && \
    cargo --version && \
    rustc --version
RUN mkdir -m 755 bin
ADD . /app
RUN cargo build --release
RUN cp /app/target/release/merino /app/bin

# =============================================================================
FROM debian:buster-slim
RUN apt-get -qq update && \
    apt-get -qq upgrade && \
    rm -rf /var/lib/apt/lists
RUN groupadd --gid 10001 app && \
    useradd --uid 10001 --gid 10001 --home /app --create-home app

COPY --from=builder /app/bin /app/bin
COPY --from=builder /app/version.json /app

ARG PORT=8080

WORKDIR /app
USER app
EXPOSE ${PORT}

CMD ["/app/bin/merino"]
