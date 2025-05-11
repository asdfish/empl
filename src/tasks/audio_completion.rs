use {
    crate::{
        either::{
            Either,
            EitherFuture,
        },
        ext::{
            future::FutureExt,
            option::OptionExt,
        },
        tasks::ChannelError,
    },
    tokio::sync::{
        mpsc,
        oneshot,
    },
};

#[derive(Debug)]
pub struct AudioCompletionTask {
    change_completion_notifier_rx: mpsc::UnboundedReceiver<oneshot::Receiver<()>>,
    completion_rx: Option<oneshot::Receiver<()>>,
    completion_tx: mpsc::UnboundedSender<()>,
}
impl AudioCompletionTask {
    pub const fn new(change_completion_notifier_rx: mpsc::UnboundedReceiver<oneshot::Receiver<()>>, completion_tx: mpsc::UnboundedSender<()>) -> Self {
        Self {
            change_completion_notifier_rx,
            completion_rx: None,
            completion_tx,
        }
    }

    pub async fn run<'a>(&mut self) -> Result<(), ChannelError<'a>> {
        loop {
            match EitherFuture::new(
                self.change_completion_notifier_rx.recv(),
                self.completion_rx.as_mut().maybe_future(),
            ).await {
                Either::Left(Some(completion_notifier)) => {},
                Either::Left(None) => break Err(ChannelError::ChangeCompletionNotifier(None)),
                Either::Right(Some(_)) => self.completion_tx.send(()).map_err(|_| ChannelError::AudioCompletion(Some(())))?,
                Either::Right(None) => {},
            }
        }
    }
}
