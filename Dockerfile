ARG VERSION=3.18
FROM alpine:${VERSION}
ENTRYPOINT ["marathon-cloud"]
WORKDIR "/work"
COPY target/release/marathon-cloud /usr/local/bin
