use std::{io, num::ParseIntError};

use bytes::{Bytes, BytesMut};
use thiserror::Error;
use tokio::io::{AsyncRead, BufReader};

#[derive(Debug, Error)]
pub enum FrameError {
    #[error("corrupt frame")]
    Corrupt,
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),
}

impl From<()> for FrameError {
    fn from(_: ()) -> Self {
        FrameError::Corrupt
    }
}

impl From<ParseIntError> for FrameError {
    fn from(_: ParseIntError) -> Self {
        FrameError::Corrupt
    }
}

pub trait TryBool {
    fn ok(&self) -> Result<(), ()>;
}

impl TryBool for bool {
    fn ok(&self) -> Result<(), ()> {
        self.then_some(()).ok_or(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Frame(Bytes);

impl From<Frame> for Bytes {
    fn from(frame: Frame) -> Self {
        frame.0
    }
}

pub(crate) struct FrameReader<R: AsyncRead + Unpin>(BufReader<R>);

impl<R: AsyncRead + Unpin> FrameReader<R> {
    pub fn new(reader: R) -> Self {
        Self(BufReader::new(reader))
    }

    pub async fn discard_mime_boundary(&mut self) -> Result<(), FrameError> {
        self.read_line()
            .await?
            .starts_with("--ffmpeg")
            .ok()
            .map_err(FrameError::from)
    }

    pub async fn read_frame(&mut self) -> Result<Frame, FrameError> {
        // Read the Content-type header, which ffmpeg emits first
        self.read_line().await?.starts_with("Content-type:").ok()?;

        // Capture the Content-length header, which ffmpeg emits second
        let len_hdr = self.read_line().await?;
        len_hdr.starts_with("Content-length:").ok()?;

        // Parse content length
        let len_str = len_hdr
            .split_ascii_whitespace()
            .last()
            .ok_or(FrameError::Corrupt)?;
        let content_length = len_str.parse::<usize>()?;

        // Discard the trailing empty line
        (self.read_line().await?.trim() == "").ok()?;

        // Read data payload
        let data = self.read_bytes(content_length).await?;

        // Ensure data is the correct length
        (data.len() == content_length).ok()?;

        // Discard the trailing empty line
        (self.read_line().await?.trim() == "").ok()?;

        // Discard the MIME boundary and emit the frame
        let boundary = self.read_line().await?;
        boundary.starts_with("--ffmpeg").ok()?;

        // Repack the frame
        let mut buf = BytesMut::with_capacity(256 * 1024);
        buf.extend(
            "Content-type: image/jpeg\r\nContent-length: "
                .bytes()
                .chain(len_str.bytes())
                .chain("\r\n\r\n".bytes())
                .chain(data)
                .chain("\r\n--ffmpeg\r\n".bytes()),
        );

        Ok(Frame(buf.freeze()))
    }

    async fn read_bytes(&mut self, count: usize) -> Result<Bytes, FrameError> {
        use tokio::io::AsyncReadExt;
        let mut buf = BytesMut::zeroed(count);
        Ok(self.0.read_exact(&mut buf).await.map(|_| buf.freeze())?)
    }

    async fn read_line(&mut self) -> Result<String, FrameError> {
        use tokio::io::AsyncBufReadExt;
        let mut line = String::new();
        Ok(self.0.read_line(&mut line).await.map(|_| line)?)
    }
}
