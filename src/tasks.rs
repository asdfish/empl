pub mod display;
pub mod event;
pub mod state;

use {
    crate::{
        config::Playlists,
        ext::future::FutureExt,
        select::select3,
        tasks::{
            display::DisplayTask,
            event::{Event, EventTask},
            state::{StateError, StateTask},
        },
    },
    std::io,
    tokio::sync::mpsc,
};

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
        [] => {},
        [tx] => **tx = new_tx,
        txs => txs.iter_mut()
            .for_each(|tx| **tx = new_tx.clone()),
    }
    *rx = new_rx;
}

#[derive(Debug)]
pub struct TaskManager<'a> {
    display: DisplayTask<'a>,
    event: EventTask,
    state: StateTask<'a>,
}
impl<'a> TaskManager<'a> {
    pub async fn new(playlists: &'a Playlists) -> Result<Self, io::Error> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let (display_tx, display_rx) = mpsc::unbounded_channel();

        Ok(Self {
            display: DisplayTask::new(display_rx).await?,
            event: EventTask::new(action_tx),
            state: StateTask::new(playlists, display_tx, action_rx),
        })
    }

    pub async fn run(&mut self) -> Result<(), io::Error> {
        loop {
            match select3(
                self.display
                    .run()
                    .pipe(|result| result.map_err(TaskError::Render)),
                self.event
                    .run()
                    .pipe(|result| result.map_err(TaskError::EventSend)),
                self.state
                    .run()
                    .pipe(|result| result.map_err(TaskError::State)),
            )
            .await
            {
                Ok(()) => break Ok(()),
                Err(TaskError::EventSend(e)) => {
                    fix_channel(&mut [&mut self.event.event_tx], &mut self.state.event_rx, Some(e.0));
                }
                Err(TaskError::State(StateError::DisplaySend(e))) => {
                    fix_channel(&mut [&mut self.state.display_tx], &mut self.display.display_rx, Some(e.0));
                }
                Err(TaskError::State(StateError::EventRecv)) => {
                    fix_channel(&mut [&mut self.event.event_tx], &mut self.state.event_rx, None);
                }
                Err(TaskError::Render(e)) => break Err(e),
            }
        }
    }
}

#[derive(Debug)]
pub enum TaskError<'a> {
    EventSend(mpsc::error::SendError<Event>),
    State(StateError<'a>),
    Render(io::Error),
}
