mod mjpeg;
use mjpeg::Process;

use std::sync::Arc;

use async_stream::stream;
use axum::{extract::State, response::IntoResponse, routing, Router, Server};
use bytes::Bytes;
use clap::Parser;
use tracing::info;

const CONTENT_TYPE: &str = "multipart/x-mixed-replace; boundary=ffmpeg";

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(
        long,
        short,
        help = "Buffer size, measured in seconds",
        default_value = "5"
    )]
    buffer: usize,

    #[arg(
        long,
        short,
        help = "Target framerate for the transcoded MJPEG stream",
        default_value = "10"
    )]
    fps: usize,

    #[arg(
        long,
        short,
        help = "Address to bind server to",
        default_value = "127.0.0.1"
    )]
    listen_addr: String,

    #[arg(long, short, help = "Port to listen on", default_value = "3000")]
    port: u16,

    #[arg(help = "Stream to transcode to MJPEG")]
    stream: String,
}

async fn handler(State(stream): State<Arc<Process>>) -> impl IntoResponse {
    use axum::{
        body::StreamBody,
        http::{header, StatusCode},
    };

    let mut stream = stream.subscribe();
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, CONTENT_TYPE),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        StreamBody::new(stream! {
            // Inject the first MIME delimiter.
            // These come at the END of frames,
            // but also need to separate the
            // status code and the first frame
            yield Ok(Bytes::from("--ffmpeg\r\n"));

            loop {
                match stream.next_frame().await {
                    Err(_) => continue,
                    x => yield x.map(Bytes::from),
                }
            }
        }),
    )
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Configure logging to stdout via `tracing`
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let transcoder = Arc::new(Process::new(args.stream, args.fps, args.buffer)?);
    let app = Router::new()
        .route("/", routing::get(handler))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(transcoder);

    let addr = format!("{}:{}", args.listen_addr, args.port);
    info!("Listening on {addr}");

    Server::bind(&addr.parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
