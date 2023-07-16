mod mjpeg;
use mjpeg::Process;

mod server;
use server::Server;

use clap::Parser;

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Configure logging to stdout via `tracing`
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let transcoder = Process::new(args.stream, args.fps, args.buffer)?;
    let server = Server::new((args.listen_addr, args.port), transcoder).await?;

    server.listen().await?;

    Ok(())
}
