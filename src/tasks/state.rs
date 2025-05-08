use {
    crate::{
        config::{KeyAction, Playlists},
        tasks::{
            audio::AudioAction,
            display::{
                damage::DamageList,
                state::{DisplayState, Focus},
            },
            event::Event,
            ChannelError,
        },
    },
    tokio::sync::mpsc,
};

#[derive(Debug)]
pub struct StateTask<'a> {
    pub audio_action_tx: mpsc::UnboundedSender<AudioAction<'a>>,
    cursor_cache: Box<[u16]>,
    pub display_tx: mpsc::UnboundedSender<DamageList<'a>>,
    display_state: DisplayState<'a>,
    pub event_rx: mpsc::UnboundedReceiver<Event>,
}
impl<'a> StateTask<'a> {
    pub fn new(
        display_state: DisplayState<'a>,
        playlists: &'a Playlists,
        audio_action_tx: mpsc::UnboundedSender<AudioAction<'a>>,
        display_tx: mpsc::UnboundedSender<DamageList<'a>>,
        event_rx: mpsc::UnboundedReceiver<Event>,
    ) -> Self {
        let display_state = DisplayState::new(playlists);

        Self {
            audio_action_tx,
            cursor_cache: (0..playlists.len().get()).map(|_| 0).collect(),
            display_tx,
            display_state,
            event_rx,
        }
    }

    pub async fn run(&mut self) -> Result<(), ChannelError<'a>> {
        loop {
            self.display_tx.send(
                match self.event_rx.recv().await.ok_or(ChannelError::Event(None))? {
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
                    Event::KeyBinding(KeyAction::Select)
                        if self.display_state.focus == Focus::Songs =>
                    {
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
