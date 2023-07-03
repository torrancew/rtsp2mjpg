use super::{
    frame::{Frame, FrameError, FrameReader},
    Stream, Transcoder,
};

use std::{io, process::Stdio};

use async_process::{Child, Command};
use thiserror::Error;
use tokio::{
    sync::broadcast::{error::SendError, Sender},
    task::JoinHandle,
};

#[derive(Debug, Error)]
pub(crate) enum ProcessError {
    #[error("xmit error: {0}")]
    Channel(#[from] SendError<Frame>),
    #[error("frame error: {0}")]
    Frame(#[from] FrameError),
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),
    #[error("pipe error")]
    Pipe,
}

pub(crate) struct Process {
    channel: Sender<Frame>,
    #[allow(dead_code)]
    handle: JoinHandle<Result<(), ProcessError>>,
    #[allow(dead_code)]
    process: Child,
}

impl Process {
    pub fn new(
        source: impl AsRef<str>,
        fps: usize,
        buffer_secs: usize,
    ) -> Result<Self, ProcessError> {
        let buffered_frames = fps * buffer_secs;
        let (channel, _) = tokio::sync::broadcast::channel(buffered_frames);
        let mut process = Command::new("ffmpeg")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .kill_on_drop(true)
            .args([
                "-rtsp_transport",
                "tcp",
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
            ])
            .spawn()?;

        let mut reader = process
            .stdout
            .take()
            .ok_or(ProcessError::Pipe)
            .map(FrameReader::new)?;

        let tx = channel.clone();
        let handle = tokio::spawn(async move {
            // Discard the leading MIME boundary before looping over the incoming frames
            reader.discard_mime_boundary().await?;

            // Enter the streaming loop
            // TODO: add a configurable timeout to handle upstream streams disappearing
            // TODO: halt the stream if the child process fails
            loop {
                match reader.read_frame().await {
                    Err(FrameError::Corrupt) => continue,
                    Ok(frame) => {
                        let _ = tx.send(frame);
                        continue;
                    }
                    Err(e) => return Err(ProcessError::from(e)),
                }
            }
        });

        Ok(Self {
            channel,
            handle,
            process,
        })
    }
}

impl Transcoder for Process {
    type Error = ProcessError;
    type Output = Stream;

    fn start(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn stop(&self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn subscribe(&self) -> Self::Output {
        Stream(self.channel.subscribe())
    }
}
