use {
    crate::tasks::{ChannelError, state::Event},
    std::{
        error::Error,
        fmt::{self, Display, Formatter},
    },
    symphonia::core::errors::Error as SymphoniaError,
    tokio::sync::mpsc,
};

#[derive(Debug)]
pub enum AudioError {
    Decoder(SymphoniaError),
    NoTracks,
}
impl Display for AudioError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Decoder(e) => write!(f, "error while decoding: {e}"),
            Self::NoTracks => f.write_str("selected file contains no tracks"),
        }
    }
}
impl Error for AudioError {}

#[derive(Debug)]
pub struct AudioErrorTask {
    pub audio_error_rx: mpsc::UnboundedReceiver<AudioError>,
    pub event_tx: mpsc::UnboundedSender<Event>,
}
impl AudioErrorTask {
    pub const fn new(
        audio_error_rx: mpsc::UnboundedReceiver<AudioError>,
        event_tx: mpsc::UnboundedSender<Event>,
    ) -> Self {
        Self {
            audio_error_rx,
            event_tx,
        }
    }

    pub async fn run<'a>(&mut self) -> Result<(), ChannelError<'a>> {
        loop {
            match self
                .audio_error_rx
                .recv()
                .await
                .ok_or(ChannelError::AudioError)?
            {
                AudioError::Decoder(_) => return Err(ChannelError::AudioError),
                AudioError::NoTracks => self.event_tx.send(Event::AudioFinished)?,
            }
        }
    }
}
