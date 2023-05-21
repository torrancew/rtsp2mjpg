FROM docker.io/library/alpine:3.17
ARG arch

RUN apk add --no-cache ffmpeg
COPY ./rtsp2mjpg.$arch /usr/local/bin/rtsp2mjpg
RUN chmod 0755 /usr/local/bin/rtsp2mjpg

ENTRYPOINT ["/usr/local/bin/rtsp2mjpg"]
