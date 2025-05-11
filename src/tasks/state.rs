use {
    crate::{
        config::{KeyAction, Playlists},
        ext::future::FutureExt,
        select::Select,
        tasks::{
            audio::AudioAction,
            display::{
                damage::DamageList,
                state::{Area, DisplayState, Focus, Marker},
            },
            ChannelError,
        },
    },
    fastrand::Rng,
    std::{
        num::NonZeroU16,
        sync::Arc,
        time::{SystemTime, UNIX_EPOCH},
    },
    tokio::sync::mpsc,
};

#[derive(Debug)]
pub struct StateTask<'a> {
    cursor_cache: Box<[u16]>,
    pub audio_action_tx: mpsc::Sender<AudioAction>,
    pub audio_completion_rx: mpsc::Receiver<()>,
    pub display_tx: mpsc::Sender<DamageList<'a>>,
    display_state: DisplayState<'a>,
    pub event_rx: mpsc::Receiver<Event>,
    rng: Rng,
}
impl<'a> StateTask<'a> {
    pub fn new(
        display_state: DisplayState<'a>,
        playlists: &'a Playlists,
        audio_action_tx: mpsc::Sender<AudioAction>,
        audio_completion_rx: mpsc::Receiver<()>,
        display_tx: mpsc::Sender<DamageList<'a>>,
        event_rx: mpsc::Receiver<Event>,
    ) -> Self {
        Self {
            cursor_cache: (0..playlists.len().get()).map(|_| 0).collect(),
            audio_action_tx,
            audio_completion_rx,
            display_tx,
            display_state,
            event_rx,
            rng: Rng::with_seed(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_else(|err| err.duration())
                    .as_secs(),
            ),
        }
    }

    async fn play(&mut self, song_index: u16) -> Option<Result<DamageList<'a>, ChannelError<'a>>> {
        let Some(path) = self
            .display_state
            .playlists()
            .get(usize::from(self.display_state.selected_menu))
            .and_then(|(_, playlist)| playlist.get(usize::from(song_index)))
            .map(|(_, path)| Arc::clone(path))
        else {
            return None;
        };
        if let Err(err) = self.audio_action_tx.send(AudioAction::Play(path)).await {
            return Some(Err(ChannelError::from(err)));
        }

        Some(Ok(self.display_state.write(|state| {
            state.selected_song.index = song_index;
        })))
    }

    pub async fn run(&mut self) -> Result<(), ChannelError<'a>> {
        loop {
            let damage_list = match Select::new(
                self.event_rx
                    .recv()
                    .pipe(|event| event.ok_or(ChannelError::Event(None))),
                self.audio_completion_rx.recv().pipe(|event| {
                    event
                        .map(|_| Event::AudioFinished)
                        .ok_or(ChannelError::AudioCompletion(None))
                }),
            )
            .await?
            {
                Event::AudioFinished | Event::KeyBinding(KeyAction::SkipSong) => {
                    let Some(len) = self
                        .display_state
                        .playlists()
                        .get(usize::from(self.display_state.selected_menu))
                        .map(|playlist| playlist.1.len())
                    else {
                        continue;
                    };
                    loop {
                        let song = self.rng.u16(..);

                        match self
                            .play(song % NonZeroU16::try_from(len).unwrap_or(NonZeroU16::MAX))
                            .await
                            .transpose()?
                        {
                            Some(dl) => break dl,
                            None => continue,
                        }
                    }
                }
                Event::KeyBinding(KeyAction::Quit) => break Ok(()),
                Event::KeyBinding(KeyAction::MoveUp(n)) => self.display_state.write(|state| {
                    state.cursors[state.focus] = state.cursors[state.focus].saturating_sub(n);
                }),
                Event::KeyBinding(KeyAction::MoveDown(n)) => self.display_state.write(|state| {
                    state.cursors[state.focus] = state.cursors[state.focus].saturating_add(n);
                }),
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
                    if self.display_state.focus == Focus::Songs
                        && self.display_state.cursors[Focus::Songs]
                            != self.display_state.selected_song.index =>
                {
                    match self
                        .play(self.display_state.cursors[Focus::Songs])
                        .await
                        .transpose()?
                    {
                        Some(dl) => dl,
                        None => continue,
                    }
                }
                Event::KeyBinding(KeyAction::Select) => continue,
                Event::KeyBinding(KeyAction::MoveBottom) => self.display_state.write(|state| {
                    if let Some(len) = state.len(state.focus) {
                        state.cursors[state.focus] = u16::try_from(len.get()).unwrap_or(u16::MAX);
                    }
                }),
                Event::KeyBinding(KeyAction::MoveTop) => self.display_state.write(|state| {
                    state.cursors[state.focus] = 0;
                }),
                Event::KeyBinding(KeyAction::MoveSelection) => self.display_state.write(|state| {
                    state.cursors[state.focus] = Marker::Selection.get(state.focus, state);
                }),
                Event::Resize(area) => self.display_state.write(move |state| {
                    state.terminal_area = Some(area);
                }),
            };

            self.display_tx.send(damage_list).await?;
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Event {
    AudioFinished,
    KeyBinding(KeyAction),
    Resize(Area),
}
