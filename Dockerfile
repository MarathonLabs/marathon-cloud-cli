ARG RUST_VERSION=1.92.0
ARG ALPINE_VERSION=3.18
FROM rust:${RUST_VERSION}-alpine AS build
ARG TARGETARCH
RUN apk update && apk add --no-cache musl-dev
WORKDIR /usr/src/
COPY . .
RUN if [ "$TARGETARCH" = "amd64" ]; then TARGET="x86_64-unknown-linux-musl"; elif [ "$TARGETARCH" = "arm64" ]; then TARGET="aarch64-unknown-linux-musl"; else echo "$TARGETARCH"; exit 1; fi && \
  cargo install --target=$TARGET --path . && \
  /usr/local/cargo/bin/marathon-cloud --help

FROM alpine:${ALPINE_VERSION}
ARG TARGETARCH
COPY --from=build /usr/local/cargo/bin/marathon-cloud /usr/local/bin/
ENTRYPOINT ["marathon-cloud"]
WORKDIR "/work"
