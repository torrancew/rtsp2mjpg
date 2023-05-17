use super::frame::{Frame, FrameError, FrameReader};

use std::{ffi::OsStr, io, process::Stdio};

use thiserror::Error;
use tokio::{
    process::{Child, Command},
    sync::broadcast::{error::SendError, Receiver, Sender},
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
    pub fn new<S: AsRef<OsStr>>(
        cmd: impl AsRef<OsStr>,
        args: impl IntoIterator<Item = S>,
        channel: Sender<Frame>,
    ) -> Result<Self, ProcessError> {
        let mut process = Command::new(cmd)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(false)
            .spawn()?;

        let mut reader = process
            .stdout
            .take()
            .ok_or(ProcessError::Pipe)
            .map(FrameReader::new)?;

        let tx = channel.clone();
        let handle: JoinHandle<Result<(), ProcessError>> = tokio::spawn(async move {
            // Discard the leading MIME boundary before looping over the incoming frames
            reader.discard_mime_boundary().await?;

            // Enter the streaming loop
            // TODO: add a configurable timeout to handle upstream streams disappearing
            // TODO: halt the stream upon receipt of a signal from stopper
            loop {
                match reader.read_frame().await {
                    Err(FrameError::Corrupt) => continue,
                    Ok(frame) => match tx.send(frame) {
                        Ok(_) => continue,
                        Err(e) => return Err(ProcessError::from(e)),
                    },
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

    pub fn subscribe(&self) -> Receiver<Frame> {
        self.channel.subscribe()
    }
}
