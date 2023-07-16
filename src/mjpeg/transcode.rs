use super::{
    frame::{DeadStream, Frame, FrameReader},
    Stream, Transcoder,
};

use std::{io, process::Stdio};

use async_broadcast::{InactiveReceiver, Sender, TrySendError};
use async_process::{Child, Command};
use smol::Task;
use thiserror::Error;

#[derive(Debug, Error)]
pub(crate) enum ProcessError {
    #[error("channel is closed")]
    Channel,
    #[error("i/o error: {0}")]
    Io(#[from] io::Error),
    #[error("pipe error")]
    Pipe,
}

pub(crate) struct Process {
    transmitter: Sender<Result<Frame, DeadStream>>,
    #[allow(dead_code)]
    receiver: InactiveReceiver<Result<Frame, DeadStream>>,
    #[allow(dead_code)]
    handle: Task<Result<(), ProcessError>>,
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
        let (mut tx, rx) = async_broadcast::broadcast(buffered_frames);
        tx.set_overflow(true);
        tx.set_await_active(false);

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

        let channel = tx.clone();
        let handle = smol::spawn(async move {
            // Discard the leading MIME boundary before looping over the incoming frames
            let _ = reader.discard_mime_boundary().await?;

            // Enter the streaming loop
            // TODO: add a configurable timeout to handle upstream streams disappearing
            // TODO: halt the stream if the child process fails
            loop {
                let frame_result = match reader.read_frame().await {
                    Ok(None) => continue,
                    Ok(Some(frame)) => Ok(frame),
                    Err(e) => Err(e),
                };

                if let Err(TrySendError::Closed(_)) = channel.try_broadcast(frame_result) {
                    break Err(ProcessError::Channel);
                }
            }
        });

        Ok(Self {
            transmitter: tx,
            receiver: rx.deactivate(),
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
        Stream(self.transmitter.new_receiver())
    }
}
