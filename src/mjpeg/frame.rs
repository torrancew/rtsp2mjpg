use std::io;

use bytes::{Bytes, BytesMut};
use futures::io::{AsyncRead, BufReader};
use thiserror::Error;

#[derive(Clone, Debug, Error)]
#[error("dead stream")]
pub struct DeadStream;

impl From<io::Error> for DeadStream {
    fn from(_: io::Error) -> Self {
        Self
    }
}

#[allow(clippy::from_over_into)]
impl Into<io::Error> for DeadStream {
    fn into(self) -> io::Error {
        io::Error::new(io::ErrorKind::BrokenPipe, self.to_string())
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

impl AsRef<[u8]> for Frame {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

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

    async fn discard_line_with_prefix(
        &mut self,
        pfx: impl AsRef<str>,
    ) -> io::Result<Result<(), String>> {
        let line = self.read_line().await?;
        Ok(line.starts_with(pfx.as_ref()).then_some(()).ok_or(line))
    }

    pub async fn discard_mime_boundary(&mut self) -> io::Result<Result<(), String>> {
        self.discard_line_with_prefix("--ffmpeg").await
    }

    pub async fn read_frame(&mut self) -> Result<Option<Frame>, DeadStream> {
        let corrupt = Ok(None);

        // Read the Content-type header, which ffmpeg emits first
        let _ = self.discard_line_with_prefix("Content-type:").await?;

        // Capture the Content-length header, which ffmpeg emits second
        let len_hdr = self.read_line().await?;
        if len_hdr.starts_with("Content-length:").ok().is_err() {
            return corrupt;
        }

        // Parse content length
        let content_length = match len_hdr.split_ascii_whitespace().last() {
            None => return corrupt,
            Some(len_str) => match len_str.parse::<usize>() {
                Ok(len) => len,
                _ => return corrupt,
            },
        };

        // Discard the trailing empty line
        if (self.read_line().await?.trim() == "").ok().is_err() {
            return corrupt;
        }

        // Read data payload
        let data = self.read_bytes(content_length).await?;

        // Ensure data is the correct length
        if (data.len() == content_length).ok().is_err() {
            return corrupt;
        }

        // Discard the trailing empty line
        if (self.read_line().await?.trim() == "").ok().is_err() {
            return corrupt;
        }

        // Discard the MIME boundary and emit the frame
        if self.discard_mime_boundary().await?.is_err() {
            return corrupt;
        }

        // Repack the frame
        let mut buf = BytesMut::with_capacity(256 * 1024);
        buf.extend(
            "Content-type: image/jpeg\r\nContent-length: "
                .bytes()
                .chain(content_length.to_string().bytes())
                .chain("\r\n\r\n".bytes())
                .chain(data)
                .chain("\r\n--ffmpeg\r\n".bytes()),
        );

        Ok(Some(Frame(buf.freeze())))
    }

    async fn read_bytes(&mut self, count: usize) -> io::Result<Bytes> {
        use futures::io::AsyncReadExt;
        let mut buf = BytesMut::zeroed(count);
        self.0.read_exact(&mut buf).await.map(|_| buf.freeze())
    }

    async fn read_line(&mut self) -> io::Result<String> {
        use futures::io::AsyncBufReadExt;
        let mut line = String::new();
        self.0.read_line(&mut line).await.map(|_| line)
    }
}
