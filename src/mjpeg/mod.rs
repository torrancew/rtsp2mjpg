mod frame;
use frame::{DeadStream, Frame};

mod traits;
pub(crate) use traits::*;

mod transcode;
pub(crate) use transcode::Process;

use std::io;

use async_broadcast::{Receiver, RecvError};
use async_trait::async_trait;
use pin_project::pin_project;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum StreamError {
    #[error("channel error: {0}")]
    Channel(#[from] RecvError),
    #[error("dead source: {0}")]
    Source(#[from] DeadStream),
}

#[pin_project]
pub(crate) struct Stream(#[pin] Receiver<Result<Frame, DeadStream>>);

#[async_trait]
impl FrameStreamer for Stream {
    type Error = StreamError;

    async fn next_frame(&mut self) -> Result<Frame, Self::Error> {
        loop {
            match self.0.recv().await {
                Ok(result) => break Ok(result?),
                Err(e) => match e {
                    RecvError::Overflowed(_) => continue,
                    _ => return Err(StreamError::from(e)),
                },
            }
        }
    }
}

impl futures::Stream for Stream {
    type Item = io::Result<Frame>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.project().0.poll_next(cx).map_err(|e| e.into())
    }
}
