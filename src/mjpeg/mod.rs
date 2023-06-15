mod frame;
use frame::Frame;

mod traits;
pub(crate) use traits::*;

mod transcode;
pub(crate) use transcode::Process;
pub(crate) use transcode::ProcessError;

use async_trait::async_trait;
use thiserror::Error;
use tokio::sync::broadcast::{error::RecvError, Receiver};

#[derive(Debug, Error)]
pub(crate) enum StreamError {
    #[error("channel error: {0}")]
    Channel(#[from] RecvError),
    #[error("stream error: {0}")]
    Stream(#[from] ProcessError),
}

pub(crate) struct Stream(Receiver<Frame>);

#[async_trait]
impl FrameStreamer for Stream {
    type Error = StreamError;

    async fn next_frame(&mut self) -> Result<Frame, Self::Error> {
        self.0.recv().await.map_err(StreamError::from)
    }
}
