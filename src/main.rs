mod mjpeg;
use mjpeg::{Stream, StreamError};

use async_stream::stream;
use axum::{extract::State, response::IntoResponse, routing::get, Router, Server};
use clap::Parser;
use tracing::info;

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

async fn stream(State(mut stream): State<Stream>) -> impl IntoResponse {
    use axum::{
        body::StreamBody,
        http::{header, StatusCode},
    };

    use StreamError::*;

    (
        StatusCode::OK,
        [
            (
                header::CONTENT_TYPE,
                "multipart/x-mixed-replace; boundary=ffmpeg",
            ),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        StreamBody::new(stream! {
            loop {
                match stream.next_frame().await {
                    Err(Stream(_)) => continue,
                    x => yield x,
                }
            }
        }),
    )
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let transcoder = Stream::new(args.stream, args.fps, args.buffer)?;
    let app = Router::new()
        .route("/", get(stream))
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .with_state(transcoder.clone());

    let addr = format!("{}:{}", args.listen_addr, args.port);
    info!("Listening on {addr}");

    Server::bind(&addr.parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}
