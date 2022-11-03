FROM golang:1.18.8-alpine3.16 as build-stage
ENV GIN_MODE=release
WORKDIR /app
COPY go.mod .
COPY go.sum .
RUN go mod download
COPY . .
RUN go build -o testwise

FROM golang:1.18.8-alpine3.16 as production-stage
COPY --from=build-stage /app/testwise /usr/bin/testwise
RUN chmod +x /usr/bin/testwise
CMD ["/usr/bin/testwise"]
