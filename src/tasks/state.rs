use {
    crate::{
        config::{KeyAction, Playlists},
        tasks::{
            display::{
                damage::{Damage, DamageList},
                state::DisplayState,
            },
            event::Event,
        },
    },
    enum_map::EnumMap,
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
        let display_state = DisplayState::new(playlists);
        let _ = display_tx.send(DamageList::new(
            EnumMap::from_fn(|damage| matches!(damage, Damage::FullRedraw)),
            display_state,
            display_state,
        ));

        Self {
            display_tx,
            display_state,
            event_rx,
        }
    }

    pub async fn run(&mut self) -> Result<(), StateError<'a>> {
        loop {
            match self.event_rx.recv().await.ok_or(StateError::EventRecv)? {
                Event::KeyBinding(KeyAction::Quit) => break Ok(()),
                Event::KeyBinding(KeyAction::MoveUp(n)) => {
                    self.display_tx.send(self.display_state.write(|state| {
                        state.cursors[state.focus] = state.cursors[state.focus].saturating_sub(n);
                    }))?
                }
                Event::KeyBinding(KeyAction::MoveDown(n)) => {
                    self.display_tx.send(self.display_state.write(|state| {
                        state.cursors[state.focus] = state.cursors[state.focus].saturating_add(n);
                    }))?
                }
                Event::Resize(area) => {
                    self.display_tx
                        .send(self.display_state.write(move |state| {
                            state.terminal_area = Some(area);
                        }))?
                }
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
impl<'a> From<mpsc::error::SendError<DamageList<'a>>> for StateError<'a> {
    fn from(err: mpsc::error::SendError<DamageList<'a>>) -> Self {
        Self::DisplaySend(err)
    }
}
