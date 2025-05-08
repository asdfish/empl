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

#[derive(Debug)]
pub struct TaskManager<'a> {
    display: DisplayTask<'a>,
    event: EventTask,
    state: StateTask<'a>,
}
impl<'a> TaskManager<'a> {
    pub fn new(playlists: &'a Playlists) -> Self {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        let (display_tx, display_rx) = mpsc::unbounded_channel();

        Self {
            display: DisplayTask::new(display_rx),
            event: EventTask::new(action_tx),
            state: StateTask::new(playlists, display_tx, action_rx),
        }
    }

    pub async fn run(&mut self) -> Result<(), TaskError<'a>> {
        loop {
            match select3(
                self.display.run().pipe(|result| result.map_err(TaskError::Render)),
                self.event.run().pipe(|result| result.map_err(TaskError::EventSend)),
                self.state.run().pipe(|result| result.map_err(TaskError::State)),
            ) {
                _ => {}
            }
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum TaskError<'a> {
    EventSend(mpsc::error::SendError<Event>),
    State(StateError<'a>),
    Render(io::Error),
}
