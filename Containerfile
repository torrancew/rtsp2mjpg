FROM docker.io/library/rust:1.70-alpine as builder

RUN apk add --no-cache musl-dev
WORKDIR /usr/src/rtsp2mjpg
COPY . .
RUN cargo install --path .

FROM docker.io/library/alpine:3.18
RUN apk add --no-cache ffmpeg
COPY --from=builder /usr/local/cargo/bin/rtsp2mjpg /usr/local/bin/rtsp2mjpg

EXPOSE 3000

ENTRYPOINT ["/usr/local/bin/rtsp2mjpg"]
