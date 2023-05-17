# rtsp2mjpg

A small media server which uses `ffmpeg` to transcode the provided source stream
into MJPEG and serve it via HTTP to one or more clients. The client connections
share a configurably-sized buffer of re-encoded frames, and should scale fairly
well.

# License

This project is available under the MIT license. Please see the included LICENSE
for the complete license text.
