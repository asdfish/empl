pub mod audio;
pub mod audio_error;
pub mod display;
pub mod state;
pub mod terminal_event;

use {
    crate::{
        config::Playlists,
        ext::command::{CommandChain, CommandExt},
        select::Select3,
        tasks::{
            audio::{AudioAction, AudioTask},
            audio_error::AudioErrorTask,
            display::{
                DisplayTask,
                damage::{Damage, DamageList},
                state::DisplayState,
            },
            state::{Event, StateTask},
            terminal_event::TerminalEventTask,
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

pub struct TaskManager<'a> {
    audio: AudioTask,
    audio_error: AudioErrorTask,
    display: DisplayTask<'a>,
    state: StateTask<'a>,
    terminal_event: TerminalEventTask<'a>,
}
impl<'a> TaskManager<'a> {
    pub async fn new(playlists: &'a Playlists) -> Result<Self, io::Error> {
        let (audio_action_tx, audio_action_rx) = mpsc::unbounded_channel();
        let (audio_error_tx, audio_error_rx) = mpsc::unbounded_channel();
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
            audio: AudioTask::new(audio_action_rx, audio_error_tx),
            audio_error: AudioErrorTask::new(audio_error_rx, event_tx.clone()),
            display: DisplayTask::new(alloc, stdout, display_rx),
            state: StateTask::new(
                display_state,
                playlists,
                audio_action_tx,
                display_tx,
                event_rx,
            ),
            terminal_event: TerminalEventTask::new(event_tx),
        })
    }
    fn recover_audio(&mut self, msg: Option<AudioAction>) {
        let (new_audio_action_tx, new_audio_action_rx) = mpsc::unbounded_channel();
        let (new_audio_error_tx, new_audio_error_rx) = mpsc::unbounded_channel();
        if let Some(msg) = msg {
            let result = new_audio_action_tx.send(msg);
            debug_assert!(result.is_ok());
        }

        self.audio.reset(new_audio_action_rx, new_audio_error_tx);
        self.audio_error.audio_error_rx = new_audio_error_rx;
        self.state.audio_action_tx = new_audio_action_tx;
    }
    fn recover(&mut self, err: ChannelError<'a>) {
        fn recover_channel<T>(
            tx: &mut [&mut mpsc::UnboundedSender<T>],
            rx: &mut mpsc::UnboundedReceiver<T>,
            msg: Option<T>,
        ) {
            let (new_tx, new_rx) = mpsc::unbounded_channel();
            if let Some(msg) = msg {
                let result = new_tx.send(msg);
                debug_assert!(result.is_ok());
            }
            match tx {
                [] => {}
                [tx] => **tx = new_tx,
                txs => txs.iter_mut().for_each(|tx| **tx = new_tx.clone()),
            }
            *rx = new_rx;
        }

        match err {
            ChannelError::Audio(msg) => {
                self.recover_audio(msg);
            }
            ChannelError::AudioError => self.recover_audio(None),
            ChannelError::Event(msg) => recover_channel(
                &mut [
                    &mut self.audio_error.event_tx,
                    &mut self.terminal_event.event_tx,
                ],
                &mut self.state.event_rx,
                msg,
            ),
            ChannelError::Display(msg) => recover_channel(
                &mut [&mut self.state.display_tx],
                &mut self.display.display_rx,
                msg,
            ),
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            if let Err(err) = self.audio.spawn() {
                match err {
                    TaskError::Channel(err) => {
                        self.recover(err);
                        continue;
                    }
                    TaskError::OutputDevice(err) => break Err(err),
                }
            }

            match Select3::new(
                self.display.run(),
                self.state.run(),
                self.terminal_event.run(),
            )
            .await
            {
                Ok(()) => break Ok(()),
                Err(err) => self.recover(err),
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
    Audio(Option<AudioAction>),
    AudioError,
    Event(Option<Event>),
    Display(Option<DamageList<'a>>),
}
impl ChannelError<'_> {
    const fn as_str(&self) -> &str {
        match self {
            Self::Audio(_) => "audio",
            Self::AudioError => "audio error",
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

#[derive(Debug)]
pub enum TaskError<'a> {
    Channel(ChannelError<'a>),
    OutputDevice(Box<dyn Error>),
}
impl<'a> From<ChannelError<'a>> for TaskError<'a> {
    fn from(err: ChannelError<'a>) -> Self {
        Self::Channel(err)
    }
}
impl From<Box<dyn Error>> for TaskError<'_> {
    fn from(err: Box<dyn Error>) -> Self {
        Self::OutputDevice(err)
    }
}
