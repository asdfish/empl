pub mod decoder;
pub mod display;
pub mod player;
pub mod state;
pub mod terminal_event;

use {
    crate::{
        config::Playlists,
        ext::command::{CommandChain, CommandExt},
        select::Select3,
        tasks::{
            decoder::DecoderAction,
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
        sync::{Arc, mpsc as std_mpsc},
    },
    symphonia::core::audio::SampleBuffer,
    tokio::{io::AsyncWriteExt, sync::mpsc as tokio_mpsc},
};

pub struct TaskManager<'a> {
    display: DisplayTask<'a>,
    state: StateTask<'a>,
    terminal_event: TerminalEventTask,
}
impl<'a> TaskManager<'a> {
    pub async fn new(playlists: &'a Playlists) -> Result<Self, io::Error> {
        let (audio_action_tx, audio_action_rx) = tokio_mpsc::unbounded_channel();
        audio_action_tx.send(DecoderAction::Play(Arc::clone(&playlists.first().1.first().1))).unwrap();
        // let (audio_error_tx, audio_error_rx) = tokio_mpsc::unbounded_channel();
        let (event_tx, event_rx) = tokio_mpsc::unbounded_channel();
        let (display_tx, display_rx) = tokio_mpsc::unbounded_channel();
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
    fn recover(&mut self, err: ChannelError<'a>) {
        fn recover_channel<T>(
            tx: &mut [&mut tokio_mpsc::UnboundedSender<T>],
            rx: &mut tokio_mpsc::UnboundedReceiver<T>,
            msg: Option<T>,
        ) {
            let (new_tx, new_rx) = tokio_mpsc::unbounded_channel();
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
            ChannelError::Event(msg) => recover_channel(
                &mut [&mut self.terminal_event.event_tx],
                &mut self.state.event_rx,
                msg,
            ),
            ChannelError::Display(msg) => recover_channel(
                &mut [&mut self.state.display_tx],
                &mut self.display.display_rx,
                msg,
            ),
            _ => todo!("recover audio"),
        }
    }

    pub async fn run(&mut self) {
        loop {
            match Select3::new(
                self.display.run(),
                self.state.run(),
                self.terminal_event.run(),
            )
            .await
            {
                Ok(()) => break,
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
    DecoderAction(Option<DecoderAction>),
    DecoderIdle,
    DecoderOutput,
    Event(Option<Event>),
    Display(Option<DamageList<'a>>),
}
impl ChannelError<'_> {
    const fn as_str(&self) -> &str {
        match self {
            Self::DecoderAction(_) => "decoder action",
            Self::DecoderIdle => "decoder idle",
            Self::DecoderOutput => "decoder output",
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
impl From<tokio_mpsc::error::SendError<DecoderAction>> for ChannelError<'_> {
    fn from(err: tokio_mpsc::error::SendError<DecoderAction>) -> Self {
        Self::DecoderAction(Some(err.0))
    }
}
impl<'a> From<tokio_mpsc::error::SendError<DamageList<'a>>> for ChannelError<'a> {
    fn from(err: tokio_mpsc::error::SendError<DamageList<'a>>) -> Self {
        Self::Display(Some(err.0))
    }
}
impl From<tokio_mpsc::error::SendError<Event>> for ChannelError<'_> {
    fn from(err: tokio_mpsc::error::SendError<Event>) -> Self {
        Self::Event(Some(err.0))
    }
}
impl From<std_mpsc::SendError<SampleBuffer<f32>>> for ChannelError<'_> {
    fn from(_: std_mpsc::SendError<SampleBuffer<f32>>) -> Self {
        Self::DecoderOutput
    }
}
