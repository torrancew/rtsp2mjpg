.PHONY: all binary-amd64 binary-arm64 build clean image image-amd64 image-arm64

BINARY_AMD64=target/x86_64-unknown-linux-musl/release/rtsp2mjpg
BINARY_ARM64=target/aarch64-unknown-linux-musl/release/rtsp2mjpg

all: build

build:
	cargo build --release

$(BINARY_ARM64):
	cargo build --release --target aarch64-unknown-linux-musl

$(BINARY_AMD64):
	cross build --release --target x86_64-unknown-linux-musl

clean:
	cargo clean

image: image-amd64 image-arm64
	buildah manifest create rtsp2mjpg:latest
	buildah manifest add rtsp2mjpg:latest localhost/rtsp2mjpg-amd64:latest
	buildah manifest add rtsp2mjpg:latest localhost/rtsp2mjpg-arm64:latest

image-amd64: $(BINARY_AMD64)
	@cp $< ./rtsp2mjpg.x86_64
	buildah build -f Containerfile --layers --arch amd64 -t rtsp2mjpg-amd64:latest --build-arg arch=x86_64
	$(RM) ./rtsp2mjpg.x86_64

image-arm64: $(BINARY_ARM64)
	@cp $< ./rtsp2mjpg.aarch64
	buildah build -f Containerfile --layers --arch arm64 -t rtsp2mjpg-arm64:latest --build-arg arch=aarch64
	$(RM) ./rtsp2mjpg.aarch64
