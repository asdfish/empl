use {
    crate::{
        config::{KeyAction, Playlists},
        display::{damage::DamageList, state::DisplayState},
        tasks::event::Event,
    },
    std::{
        error::Error,
        fmt::{self, Display, Formatter},
    },
    tokio::sync::mpsc,
};

#[derive(Debug)]
pub struct StateTask<'a> {
    pub display_tx: mpsc::UnboundedSender<DamageList<'a>>,
    display_state: DisplayState<'a>,
    pub event_rx: mpsc::UnboundedReceiver<Event>,
}
impl<'a> StateTask<'a> {
    pub fn new(
        playlists: &'a Playlists,
        display_tx: mpsc::UnboundedSender<DamageList<'a>>,
        event_rx: mpsc::UnboundedReceiver<Event>,
    ) -> Self {
        Self {
            display_tx,
            display_state: DisplayState::new(playlists),
            event_rx,
        }
    }

    pub async fn run(&mut self) -> Result<(), StateError<'a>> {
        #[expect(clippy::never_loop)]
        loop {
            match self.event_rx.recv().await.ok_or(StateError::EventRecv)? {
                Event::KeyBinding(KeyAction::Quit) => break Ok(()),
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum StateError<'a> {
    DisplaySend(mpsc::error::SendError<DamageList<'a>>),
    EventRecv,
}
impl Display for StateError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::DisplaySend(e) => write!(f, "failed to send display instructions: {e}"),
            Self::EventRecv => f.write_str("event channel closed unexpectedly"),
        }
    }
}
impl Error for StateError<'_> {}
