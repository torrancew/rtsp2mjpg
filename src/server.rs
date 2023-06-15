use crate::mjpeg::{FrameStreamer, Transcoder};

use std::{future::Future, io, sync::Arc};

use async_stream::stream;
use axum::{extract::State, response::IntoResponse, routing, Router};
use bytes::Bytes;
use pin_project_lite::pin_project;
use thiserror::Error;
use tokio::net::{lookup_host, ToSocketAddrs};
use tracing::info;

type AxumServer = axum::Server<hyper::server::conn::AddrIncoming, routing::IntoMakeService<Router>>;

const CONTENT_TYPE: &str = "multipart/x-mixed-replace; boundary=ffmpeg";

#[derive(Debug, Error)]
pub enum ServerError {
    #[error("hyper error: {0}")]
    Hyper(#[from] hyper::Error),
    #[error("invalid bind address")]
    InvalidAddr,
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),
}

impl From<()> for ServerError {
    fn from(_: ()) -> Self {
        Self::InvalidAddr
    }
}

pin_project! {
    pub(crate) struct Server {
        #[pin]
        inner: AxumServer,
    }
}

// `axum::Server` implements Future, and we want to expose that interface
impl Future for Server {
    type Output = Result<(), ServerError>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        Future::poll(self.project().inner, cx).map_err(ServerError::from)
    }
}

impl Server {
    pub async fn new<A, T: Transcoder + Send + Sync + 'static>(
        addr: A,
        state: T,
    ) -> Result<Self, ServerError>
    where
        A: ToSocketAddrs,
        <T as Transcoder>::Output: Send + 'static,
    {
        use axum::Server;

        let addr = lookup_host(addr).await?.next().ok_or(())?;
        info!("Listening on {addr}");

        let app = Router::new()
            .route("/", routing::get(handler))
            .layer(tower_http::trace::TraceLayer::new_for_http())
            .with_state(Arc::new(state));

        Ok(Server::try_bind(&addr).map(|b| Self {
            inner: b.serve(app.into_make_service()),
        })?)
    }
}

async fn handler<T>(State(stream): State<Arc<T>>) -> impl IntoResponse
where
    T: Transcoder,
    <T as Transcoder>::Output: Send + 'static,
{
    use axum::{
        body::StreamBody,
        http::{header, StatusCode},
    };

    info!("subscribing to stream for new incoming connection");
    let mut stream = stream.subscribe();
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, CONTENT_TYPE),
            (header::CACHE_CONTROL, "no-cache"),
        ],
        StreamBody::new(stream! {
            // Inject the first MIME delimiter.
            // These come at the end of frames,
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
