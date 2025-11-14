FROM golang:1.23-alpine AS builder

RUN apk add --no-cache git

WORKDIR /workspace
COPY . .
RUN go mod download
RUN GOOS=wasip1 GOARCH=wasm CGO_ENABLED=0 go build -o plugin.wasm ./cmd/plugin

FROM scratch
WORKDIR /
COPY --from=builder /workspace/plugin.wasm /plugin.wasm
