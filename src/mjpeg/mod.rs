mod frame;
use frame::Frame;

mod transcode;
pub(crate) use transcode::Process;
use transcode::ProcessError;

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

impl Stream {
    pub async fn next_frame(&mut self) -> Result<Frame, StreamError> {
        self.0.recv().await.map_err(StreamError::from)
    }
}
