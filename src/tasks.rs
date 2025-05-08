pub mod audio;
pub mod display;
pub mod event;
pub mod state;

use {
    crate::{
        config::Playlists,
        ext::command::{CommandChain, CommandExt},
        select::select4,
        tasks::{
            audio::{AudioAction, AudioTask},
            display::{
                DisplayTask,
                damage::{Damage, DamageList},
                state::DisplayState,
            },
            event::{Event, EventTask},
            state::StateTask,
        },
    },
    bumpalo::Bump,
    crossterm::{
        QueueableCommand, cursor,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    enum_map::EnumMap,
    std::{
        error::Error,
        fmt::{self, Display, Formatter},
        io::{self, Write},
    },
    tokio::{io::AsyncWriteExt, sync::mpsc},
};

#[derive(Debug)]
pub struct TaskManager<'a> {
    audio: AudioTask<'a>,
    display: DisplayTask<'a>,
    event: EventTask<'a>,
    state: StateTask<'a>,
}
impl<'a> TaskManager<'a> {
    pub async fn new(playlists: &'a Playlists) -> Result<Self, io::Error> {
        let (audio_action_tx, audio_action_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (display_tx, display_rx) = mpsc::unbounded_channel();
        let display_state = DisplayState::new(playlists);
        let _ = display_tx.send(DamageList::new(
            EnumMap::from_fn(|damage| matches!(damage, Damage::FullRedraw)),
            display_state,
            display_state,
        ));

        let alloc = Bump::new();
        let mut stdout = tokio::io::stdout();
        enable_raw_mode()?;
        cursor::Hide
            .adapt()
            .then(EnterAlternateScreen.adapt())
            .execute(&alloc, &mut stdout)
            .await?;
        stdout.flush().await?;

        Ok(Self {
            audio: AudioTask::new(audio_action_rx),
            display: DisplayTask::new(alloc, stdout, display_rx),
            event: EventTask::new(event_tx),
            state: StateTask::new(
                display_state,
                playlists,
                audio_action_tx,
                display_tx,
                event_rx,
            ),
        })
    }

    pub async fn run(&mut self) {
        fn fix_channel<T>(
            tx: &mut [&mut mpsc::UnboundedSender<T>],
            rx: &mut mpsc::UnboundedReceiver<T>,
            msg: Option<T>,
        ) {
            let (new_tx, new_rx) = mpsc::unbounded_channel();
            if let Some(msg) = msg {
                let _ = new_tx.send(msg);
            }
            match tx {
                [] => {}
                [tx] => **tx = new_tx,
                txs => txs.iter_mut().for_each(|tx| **tx = new_tx.clone()),
            }
            *rx = new_rx;
        }

        loop {
            match select4(
                self.audio.run(),
                self.display.run(),
                self.event.run(),
                self.state.run(),
            )
            .await
            {
                Ok(()) => break,
                Err(ChannelError::Audio(miss)) => fix_channel(
                    &mut [&mut self.state.audio_action_tx],
                    &mut self.audio.audio_action_rx,
                    miss,
                ),
                Err(ChannelError::Event(miss)) => fix_channel(
                    &mut [&mut self.event.event_tx],
                    &mut self.state.event_rx,
                    miss,
                ),
                Err(ChannelError::Display(miss)) => fix_channel(
                    &mut [&mut self.state.display_tx],
                    &mut self.display.display_rx,
                    miss,
                ),
            }
        }
    }
}
impl Drop for TaskManager<'_> {
    fn drop(&mut self) {
        let mut stdout = std::io::stdout();
        let _ = stdout.queue(LeaveAlternateScreen);
        let _ = stdout.queue(cursor::Show);
        let _ = stdout.flush();
        let _ = disable_raw_mode();
    }
}

#[derive(Debug)]
pub enum ChannelError<'a> {
    Audio(Option<AudioAction<'a>>),
    Event(Option<Event>),
    Display(Option<DamageList<'a>>),
}
impl ChannelError<'_> {
    const fn as_str(&self) -> &str {
        match self {
            Self::Audio(_) => "audio",
            Self::Event(_) => "event",
            Self::Display(_) => "display",
        }
    }
}
impl Display for ChannelError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "{} channel closed unexpectedly", self.as_str())
    }
}
impl Error for ChannelError<'_> {}
impl<'a> From<mpsc::error::SendError<DamageList<'a>>> for ChannelError<'a> {
    fn from(err: mpsc::error::SendError<DamageList<'a>>) -> Self {
        Self::Display(Some(err.0))
    }
}
impl From<mpsc::error::SendError<Event>> for ChannelError<'_> {
    fn from(err: mpsc::error::SendError<Event>) -> Self {
        Self::Event(Some(err.0))
    }
}
