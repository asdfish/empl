use {
    crate::{
        config::{KeyAction, Playlists},
        tasks::{
            display::{
                damage::{Damage, DamageList},
                state::{DisplayState, Focus},
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
    cursor_cache: Box<[u16]>,
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
            cursor_cache: (0..playlists.len().get()).map(|_| 0).collect(),
            display_tx,
            display_state,
            event_rx,
        }
    }

    pub async fn run(&mut self) -> Result<(), StateError<'a>> {
        loop {
            self.display_tx.send(
                match self.event_rx.recv().await.ok_or(StateError::EventRecv)? {
                    Event::KeyBinding(KeyAction::Quit) => break Ok(()),
                    Event::KeyBinding(KeyAction::MoveUp(n)) => self.display_state.write(|state| {
                        state.cursors[state.focus] = state.cursors[state.focus].saturating_sub(n);
                    }),
                    Event::KeyBinding(KeyAction::MoveDown(n)) => {
                        self.display_state.write(|state| {
                            state.cursors[state.focus] =
                                state.cursors[state.focus].saturating_add(n);
                        })
                    }
                    Event::KeyBinding(KeyAction::MoveLeft) => self
                        .display_state
                        .write(|state| state.focus = Focus::Playlists),
                    Event::KeyBinding(KeyAction::MoveRight) => {
                        self.display_state.write(|state| state.focus = Focus::Songs)
                    }
                    Event::KeyBinding(KeyAction::Select)
                        if self.display_state.focus == Focus::Playlists
                            && self.display_state.cursors[Focus::Playlists]
                                != self.display_state.selected_menu =>
                    {
                        self.display_state.write(|state| {
                            if let Some(cached_cursor) = self
                                .cursor_cache
                                .get_mut(usize::from(state.selected_menu))
                            {
                                *cached_cursor = state.cursors[Focus::Songs];
                            }

                            state.cursors[Focus::Songs] = self
                                .cursor_cache
                                .get(usize::from(state.cursors[Focus::Playlists]))
                                .copied()
                                .unwrap_or_default();
                            state.selected_menu = state.cursors[Focus::Playlists];
                        })
                    }
                    Event::KeyBinding(KeyAction::Select) if self.display_state.focus == Focus::Songs => {
                        self.display_state.write(|state| {
                            state.selected_song.index = state.cursors[Focus::Songs];
                        })
                    },
                    Event::KeyBinding(KeyAction::Select) => continue,
                    Event::Resize(area) => self.display_state.write(move |state| {
                        state.terminal_area = Some(area);
                    }),
                },
            )?;
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
