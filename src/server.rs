use crate::mjpeg::{Process, Transcoder};

use std::{io, sync::Arc};

use async_net::{resolve, AsyncToSocketAddrs};
use futures::{io::Cursor, AsyncReadExt, TryStreamExt};
use thiserror::Error;
use tide::{http::mime, Body, Request, Response};
use tracing::info;

const CONTENT_TYPE: &str = "multipart/x-mixed-replace; boundary=ffmpeg";
const PREAMBLE: &[u8] = b"--ffmpeg\r\n";

#[derive(Debug, Error)]
pub enum ServerError {
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

pub(crate) struct Server<A: AsyncToSocketAddrs> {
    server: tide::Server<Arc<Process>>,
    addr: A,
}

impl<A: AsyncToSocketAddrs> Server<A> {
    pub async fn new(addr: A, state: Process) -> Result<Self, ServerError>
    where
        A: AsyncToSocketAddrs,
    {
        let mut server = tide::Server::with_state(Arc::new(state));
        server.with(tide_tracing::TraceMiddleware);
        server.at("/").get(handler);

        Ok(Self { server, addr })
    }

    pub async fn listen(self) -> Result<(), ServerError> {
        let addr = resolve(self.addr)
            .await?
            .into_iter()
            .next()
            .ok_or(ServerError::InvalidAddr)?;
        Ok(self.server.listen(addr).await?)
    }
}

async fn handler(req: Request<Arc<Process>>) -> tide::Result {
    info!("subscribing to stream for new incoming connection");
    let stream = req.state().subscribe();

    Ok(Response::builder(200)
        .header("CACHE_CONTROL", "no-cache")
        .content_type(mime::Mime::from(CONTENT_TYPE))
        .body(Body::from_reader(
            Cursor::new(PREAMBLE).chain(stream.into_async_read()),
            None,
        ))
        .build())
}
