mod frame;
use frame::Frame;

mod transcode;
use transcode::{Process, ProcessError};

use std::sync::Arc;

use thiserror::Error;
use tokio::sync::broadcast::{error::RecvError, Receiver};

#[derive(Debug, Error)]
pub(crate) enum StreamError {
    #[error("channel error: {0}")]
    Channel(#[from] RecvError),
    #[error("stream error: {0}")]
    Stream(#[from] ProcessError),
}

#[allow(dead_code)]
pub(crate) struct Stream {
    channel: Receiver<Frame>,
    process: Arc<Process>,
}

impl Clone for Stream {
    fn clone(&self) -> Self {
        Self {
            channel: self.process.subscribe(),
            process: Arc::clone(&self.process),
        }
    }
}

impl Stream {
    pub fn new(
        source: impl AsRef<str>,
        fps: usize,
        buffer_secs: usize,
    ) -> Result<Self, StreamError> {
        let buffered_frames = fps * buffer_secs;
        let (tx, channel) = tokio::sync::broadcast::channel(buffered_frames);
        let process = Process::new(
            "ffmpeg",
            [
                "-i",
                source.as_ref(),
                "-c:v",
                "mjpeg",
                "-q:v",
                "1",
                "-f",
                "mpjpeg",
                "-filter_complex",
                &format!("[0:v] fps={fps}"),
                "-fps_mode",
                "drop",
                "-an",
                "-",
            ],
            tx,
        )
        .map(Arc::new)?;

        Ok(Self { channel, process })
    }

    pub async fn next_frame(&mut self) -> Result<Frame, StreamError> {
        self.channel.recv().await.map_err(StreamError::from)
    }
}
