.PHONY: all build clean image

all: build

build:
	cargo build --release

clean:
	cargo clean

image: image-amd64 image-arm64
	buildah manifest create rtsp2mjpg:latest
	buildah manifest add rtsp2mjpg:latest localhost/rtsp2mjpg-amd64:latest
	buildah manifest add rtsp2mjpg:latest localhost/rtsp2mjpg-arm64:latest

image-amd64:
	podman build -f Containerfile --layers --arch amd64 -t rtsp2mjpg-amd64:latest

image-arm64:
	podman build -f Containerfile --layers --arch arm64 -t rtsp2mjpg-arm64:latest
