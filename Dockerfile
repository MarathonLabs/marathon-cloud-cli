FROM golang:1.18.8-alpine3.16 as build-stage
ENV GIN_MODE=release
WORKDIR /app
COPY go.mod .
COPY go.sum .
RUN go mod download
COPY . .
RUN go build -o marathon-cli

FROM golang:1.18.8-alpine3.16 as production-stage
COPY --from=build-stage /app/marathon-cli /usr/bin/marathon-cli
RUN chmod +x /usr/bin/marathon-cli
CMD ["/usr/bin/marathon-cli"]
