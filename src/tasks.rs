pub mod audio;
pub mod audio_completion;
pub mod display;
pub mod state;
pub mod terminal_event;

use {
    crate::{
        config::Playlists,
        ext::{
            command::{CommandChain, CommandExt},
            future::FutureExt,
        },
        select::{Select, Select4},
        tasks::{
            audio::{AudioAction, AudioTask},
            audio_completion::AudioCompletionTask,
            display::{
                DisplayTask,
                damage::{Damage, DamageList},
                state::DisplayState,
            },
            state::{Event, StateTask},
            terminal_event::TerminalEventTask,
        },
    },
    awedio::backends::CpalBackendError as AudioBackendError,
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
        sync::Arc,
    },
    tokio::{io::{AsyncWriteExt, stdout}, sync::{mpsc, oneshot}},
};

pub struct TaskManager<'a> {
    audio: AudioTask,
    audio_completion: AudioCompletionTask,
    display: DisplayTask<'a>,
    state: StateTask<'a>,
    terminal_event: TerminalEventTask,
}
impl<'a> TaskManager<'a> {
    pub async fn new(playlists: &'a Playlists) -> Result<Self, NewTaskManagerError> {
        let (audio_action_tx, audio_action_rx) = mpsc::unbounded_channel();
        let _ = audio_action_tx.send(AudioAction::Play(Arc::clone(&playlists.first().1.first().1)));
        let (audio_completion_tx, audio_completion_rx) = mpsc::unbounded_channel();
        let (change_completion_notifier_tx, change_completion_notifier_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (display_tx, display_rx) = mpsc::unbounded_channel();
        let display_state = DisplayState::new(playlists);
        let _ = display_tx.send(DamageList::new(
            EnumMap::from_fn(|damage| matches!(damage, Damage::FullRedraw)),
            display_state,
            display_state,
        ));

        let alloc = Bump::new();
        let mut stdout = stdout();
        enable_raw_mode()?;
        cursor::Hide
            .adapt()
            .then(EnterAlternateScreen.adapt())
            .execute(&alloc, &mut stdout)
            .await?;
        stdout.flush().await?;

        Ok(Self {
            audio: AudioTask::new(audio_action_rx, change_completion_notifier_tx, audio_completion_tx.clone())?,
            audio_completion: AudioCompletionTask::new(change_completion_notifier_rx, audio_completion_tx),
            display: DisplayTask::new(alloc, stdout, display_rx),
            state: StateTask::new(
                display_state,
                playlists,
                audio_action_tx,
                audio_completion_rx,
                display_tx,
                event_rx,
            ),
            terminal_event: TerminalEventTask::new(event_tx),
        })
    }

    pub async fn run(&mut self) -> Result<(), UnrecoverableError> {
        loop {
            match Select::new(
                self.audio.run(),
                Select4::new(
                    self.audio_completion.run(),
                    self.display.run(),
                    self.state.run(),
                    self.terminal_event.run(),
                )
                    .pipe(|res| res.map_err(RecoverableError::Channel).map_err(TaskError::Recoverable)),
            )
            .await
            {
                Ok(()) => break Ok(()),
                Err(err) => err.try_recover(self)?,
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
    AudioAction(Option<AudioAction>),
    AudioCompletion(Option<()>),
    ChangeCompletionNotifier(Option<oneshot::Receiver<()>>),
    Event(Option<Event>),
    Display(Option<DamageList<'a>>),
}
impl ChannelError<'_> {
    const fn as_str(&self) -> &str {
        match self {
            Self::AudioAction(_) => "audio action",
            Self::AudioCompletion(_) => "audio completion",
            Self::ChangeCompletionNotifier(_) => "change completion notifier",
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
impl From<mpsc::error::SendError<AudioAction>> for ChannelError<'_> {
    fn from(err: mpsc::error::SendError<AudioAction>) -> Self {
        Self::AudioAction(Some(err.0))
    }
}
impl From<mpsc::error::SendError<oneshot::Receiver<()>>> for ChannelError<'_> {
    fn from(err: mpsc::error::SendError<oneshot::Receiver<()>>) -> Self {
        Self::ChangeCompletionNotifier(Some(err.0))
    }
}
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
pub enum NewTaskManagerError {
    AudioBackend(AudioBackendError),
    Setup(io::Error),
    OutputDevice(Box<dyn Error>),
}
impl Display for NewTaskManagerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::AudioBackend(e) => write!(f, "failed to create audio backend: {e}"),
            Self::Setup(e) => write!(f, "failed to set up terminal: {e}"),
            Self::OutputDevice(e) => write!(f, "failed to get output device: {e}"),
        }
    }
}
impl From<AudioBackendError> for NewTaskManagerError {
    fn from(err: AudioBackendError) -> Self {
        Self::AudioBackend(err)
    }
}
impl From<io::Error> for NewTaskManagerError {
    fn from(err: io::Error) -> Self {
        Self::Setup(err)
    }
}
impl From<Box<dyn Error>> for NewTaskManagerError {
    fn from(err: Box<dyn Error>) -> Self {
        Self::OutputDevice(err)
    }
}
impl Error for NewTaskManagerError {}

#[derive(Debug)]
pub enum TaskError<'a> {
    Recoverable(RecoverableError<'a>),
    Unrecoverable(UnrecoverableError),
}
impl<'a> TaskError<'a> {
    pub fn try_recover(self, task_manager: &mut TaskManager<'a>) -> Result<(), UnrecoverableError> {
        match self {
            Self::Recoverable(err) => Ok(err.recover(task_manager)),
            Self::Unrecoverable(err) => Err(err),
        }
    }
}
impl<'a> From<RecoverableError<'a>> for TaskError<'a> {
    fn from(err: RecoverableError<'a>) -> Self {
        Self::Recoverable(err)
    }
}
impl From<UnrecoverableError> for TaskError<'_> {
    fn from(err: UnrecoverableError) -> Self {
        Self::Unrecoverable(err)
    }
}

#[derive(Debug)]
pub enum RecoverableError<'a> {
    Channel(ChannelError<'a>),
}
impl<'a> From<ChannelError<'a>> for RecoverableError<'a> {
    fn from(err: ChannelError<'a>) -> Self {
        Self::Channel(err)
    }
}
impl<'a> RecoverableError<'a> {
    pub fn recover(self, tasks: &mut TaskManager<'a>) {
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

        match self {
            Self::Channel(ChannelError::Event(msg)) => recover_channel(
                &mut [&mut tasks.terminal_event.event_tx],
                &mut tasks.state.event_rx,
                msg,
            ),
            Self::Channel(ChannelError::Display(msg)) => recover_channel(
                &mut [&mut tasks.state.display_tx],
                &mut tasks.display.display_rx,
                msg,
            ),
            Self::Channel(_) => todo!("channel recovery"),
        }
    }
}

#[derive(Debug)]
pub enum UnrecoverableError {
    Stream,
}
impl Display for UnrecoverableError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Stream => f.write_str("failed to play stream"),
        }
    }
}
impl Error for UnrecoverableError {}
