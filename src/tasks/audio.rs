use {
    crate::tasks::ChannelError,
    std::{
        path::Path,
    },
    tokio::sync::mpsc,
};

#[derive(Debug)]
pub struct AudioTask<'a> {
    pub audio_action_rx: mpsc::UnboundedReceiver<AudioAction<'a>>,
}
impl<'a> AudioTask<'a> {
    pub fn new(audio_action_rx: mpsc::UnboundedReceiver<AudioAction<'a>>) -> Self {
        Self {
            audio_action_rx,
        }
    }

    pub async fn run(&mut self) -> Result<(), ChannelError<'a>> {
        loop {
            self.audio_action_rx.recv().await.ok_or(ChannelError::Audio(None))?;
            todo!()
        }
    }
}

#[derive(Debug)]
pub enum AudioAction<'a> {
    Play(&'a Path),
}
