use {
    crate::{
        either::{Either, EitherFuture},
        tasks::{ChannelError, state::Event},
    },
    tokio::sync::{mpsc, oneshot},
};

#[derive(Debug)]
pub struct AudioCompletionTask {
    pub change_completion_notifier_rx: mpsc::Receiver<oneshot::Receiver<()>>,
    pub completion_rx: Option<oneshot::Receiver<()>>,
    pub event_tx: mpsc::Sender<Event>,
}
impl AudioCompletionTask {
    pub const fn new(
        change_completion_notifier_rx: mpsc::Receiver<oneshot::Receiver<()>>,
        event_tx: mpsc::Sender<Event>,
    ) -> Self {
        Self {
            change_completion_notifier_rx,
            completion_rx: None,
            event_tx,
        }
    }

    pub async fn run<'a>(&mut self) -> Result<(), ChannelError<'a>> {
        loop {
            if let Some(mut completion_rx) = self.completion_rx.take() {
                match EitherFuture::new(
                    self.change_completion_notifier_rx.recv(),
                    &mut completion_rx,
                )
                .await
                {
                    Either::Left(Some(completion_rx)) => self.completion_rx = Some(completion_rx),
                    Either::Left(None) => break Err(ChannelError::ChangeCompletionNotifier(None)),
                    Either::Right(_) => self.event_tx.send(Event::AudioFinished).await?,
                }
            } else {
                self.completion_rx = Some(
                    self.change_completion_notifier_rx
                        .recv()
                        .await
                        .ok_or(ChannelError::ChangeCompletionNotifier(None))?,
                );
            }
        }
    }
}
