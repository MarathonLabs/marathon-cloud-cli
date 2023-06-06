ARG VERSION=3.18
FROM alpine:${VERSION}
ENTRYPOINT ["/marathon-cloud"]
WORKDIR "/work"
COPY marathon-cloud /
