use super::Frame;

use async_trait::async_trait;

#[async_trait]
pub(crate) trait FrameStreamer {
    type Error: std::error::Error + Send + Sync;

    async fn next_frame(&mut self) -> Result<Frame, Self::Error>;
}

pub(crate) trait Transcoder {
    type Error: std::error::Error + Send + Sync;
    type Output: FrameStreamer;

    fn start(&self) -> Result<(), Self::Error>;
    fn stop(&self) -> Result<(), Self::Error>;
    fn subscribe(&self) -> Self::Output;
}
