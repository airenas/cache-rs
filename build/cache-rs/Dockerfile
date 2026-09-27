#########################################################################################
## docker will invoke this file from ../.. dir in order to access code
#########################################################################################
FROM rust:1.97-bookworm AS builder

RUN apt-get update && \
    apt-get install -y --no-install-recommends  binutils

ARG BUILD_VERSION=0.1

ENV RUSTFLAGS="-C target-cpu=haswell -C opt-level=3 -C codegen-units=1"
ENV CARGO_PROFILE_RELEASE_DEBUG=false

WORKDIR /src/

COPY ./ /src

RUN --mount=type=cache,target=/usr/local/cargo/registry \
      CARGO_APP_VERSION=$BUILD_VERSION cargo build --release
RUN strip /src/target/release/cache-rs      
#########################################################################################
FROM ghcr.io/lvillis/salus:0.1.10 AS salus
#########################################################################################
FROM gcr.io/distroless/cc-debian12 AS runner
#########################################################################################
COPY LICENSE /licenses/LICENSE-bsd-3

ARG BUILD_VERSION=0.1

EXPOSE 8000

ENV RUST_LOG=info

LABEL org.opencontainers.image.version=$BUILD_VERSION \
      org.opencontainers.image.authors="airenass@gmail.com" \
      maintainer="airenass@gmail.com" \
      version=$BUILD_VERSION \
      release=$BUILD_VERSION \
      summary="This image is used to host cache service for accenter" \
      description="This image is used to host cache service for accenter" 

WORKDIR /app
#########################################################################################
COPY --from=salus /bin/salus /app/salus      
COPY --from=builder /src/target/release/cache-rs /app/
HEALTHCHECK --start-period=3s --interval=10s --timeout=3s --retries=3 \
  CMD ["/app/salus", "http", "--addr", "127.0.0.1:8000"]

ENTRYPOINT ["/app/cache-rs"]
