use {
    crate::{
        config::{KeyAction, Playlists},
        tasks::{
            ChannelError,
            decoder::DecoderAction,
            display::{
                damage::DamageList,
                state::{Area, DisplayState, Focus, Marker},
            },
        },
    },
    std::sync::{Arc, mpsc as std_mpsc},
    tokio::sync::mpsc as tokio_mpsc,
};

#[derive(Debug)]
pub struct StateTask<'a> {
    cursor_cache: Box<[u16]>,
    pub decoder_action_tx: std_mpsc::Sender<DecoderAction>,
    pub decoder_idle_rx: tokio_mpsc::UnboundedReceiver<()>,
    pub display_tx: tokio_mpsc::UnboundedSender<DamageList<'a>>,
    display_state: DisplayState<'a>,
    pub event_rx: tokio_mpsc::UnboundedReceiver<Event>,
}
impl<'a> StateTask<'a> {
    pub fn new(
        display_state: DisplayState<'a>,
        playlists: &'a Playlists,
        decoder_action_tx: std_mpsc::Sender<DecoderAction>,
        decoder_idle_rx: tokio_mpsc::UnboundedReceiver<()>,
        display_tx: tokio_mpsc::UnboundedSender<DamageList<'a>>,
        event_rx: tokio_mpsc::UnboundedReceiver<Event>,
    ) -> Self {
        Self {
            cursor_cache: (0..playlists.len().get()).map(|_| 0).collect(),
            decoder_action_tx,
            decoder_idle_rx,
            display_tx,
            display_state,
            event_rx,
        }
    }

    pub async fn run(&mut self) -> Result<(), ChannelError<'a>> {
        loop {
            self.display_tx.send(
                match self
                    .event_rx
                    .recv()
                    .await
                    .ok_or(ChannelError::Event(None))?
                {
                    Event::AudioFinished => continue,
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
                            if let Some(cached_cursor) =
                                self.cursor_cache.get_mut(usize::from(state.selected_menu))
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
                    Event::KeyBinding(KeyAction::MoveBottom) => self.display_state.write(|state| {
                        if let Some(len) = state.len(state.focus) {
                            state.cursors[state.focus] =
                                u16::try_from(len.get()).unwrap_or(u16::MAX);
                        }
                    }),
                    Event::KeyBinding(KeyAction::MoveTop) => self.display_state.write(|state| {
                        state.cursors[state.focus] = 0;
                    }),
                    Event::KeyBinding(KeyAction::MoveSelection) => {
                        self.display_state.write(|state| {
                            state.cursors[state.focus] = Marker::Selection.get(state.focus, state);
                        })
                    }
                    Event::KeyBinding(KeyAction::Select)
                        if self.display_state.focus == Focus::Songs =>
                    {
                        let Some(path) = self.display_state.playlists().get(usize::from(self.display_state.selected_menu))
                            .and_then(|(_, playlist)| playlist.get(usize::from(self.display_state.cursors[Focus::Songs])))
                            .map(|(_, path)| Arc::clone(&path)) else {
                            continue;
                        };
                        self.decoder_action_tx.send(DecoderAction::Play(path))?;

                        self.display_state.write(|state| {
                            state.selected_song.index = state.cursors[Focus::Songs];
                        })
                    }
                    Event::KeyBinding(KeyAction::Select) => continue,
                    Event::Resize(area) => self.display_state.write(move |state| {
                        state.terminal_area = Some(area);
                    }),
                },
            )?;
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Event {
    AudioFinished,
    KeyBinding(KeyAction),
    Resize(Area),
}
