use {
    awedio::{
        backends::{
            CpalBackend as Backend,
            CpalBackendError as AudioBackendError,
        },
        manager::Manager,
        Sound,
        sounds,
    },
    crate::{
        either::{
            Either,
            EitherFuture,
        },
        tasks::{
            ChannelError,
            RecoverableError,
            TaskError,
            UnrecoverableError,
        },
    },
    std::{
        path::Path,
        sync::Arc,
    },
    tokio::sync::{
        mpsc,
        oneshot,
    },
};

#[derive(Debug)]
pub enum AudioAction {
    Play(Arc<Path>),
}

pub struct AudioTask {
    action_rx: mpsc::UnboundedReceiver<AudioAction>,
    change_completion_notifier_tx: mpsc::UnboundedSender<oneshot::Receiver<()>>,
    completion_notifier_tx: mpsc::UnboundedSender<()>,
    error_rx: mpsc::UnboundedReceiver<()>,
    manager: Manager,
    _backend: Backend,
}
impl AudioTask {
    pub fn new(
        action_rx: mpsc::UnboundedReceiver<AudioAction>,
        change_completion_notifier_tx: mpsc::UnboundedSender<oneshot::Receiver<()>>,
        completion_notifier_tx: mpsc::UnboundedSender<()>,
    ) -> Result<Self, AudioBackendError> {
        let mut backend = Backend::with_defaults().ok_or(AudioBackendError::NoDevice)?;
        let (error_tx, error_rx) = mpsc::unbounded_channel();
        let mut error_tx = error_tx;

        Ok(Self {
            action_rx,
            change_completion_notifier_tx,
            completion_notifier_tx,
            error_rx,
            manager: backend
                .start(move |_| {
                    let _ = error_tx.send(());
                })?,
            _backend: backend,
        })
    }

    fn play<'a, P>(&mut self, path: &P) -> Result<(), ChannelError<'a>>
    where P: AsRef<Path> + ?Sized {
        let (sound, completion_notifier) = sounds::open_file(path)
            .map_err(|_| ChannelError::AudioCompletion(Some(())))?
            .with_async_completion_notifier();
        self.change_completion_notifier_tx.send(completion_notifier)?;

        self
            .manager
            .play(Box::new(sound));
        Ok(())
    }

    pub async fn run<'a>(&mut self) -> Result<(), TaskError<'a>> {
        loop {
            match EitherFuture::new(self.action_rx.recv(), self.error_rx.recv()).await {
                Either::Left(Some(AudioAction::Play(path))) => {
                    self.play(&path)
                        .map_err(RecoverableError::Channel)
                        .map_err(TaskError::Recoverable)?;
                },
                Either::Left(None) => break Err(TaskError::Recoverable(RecoverableError::Channel(ChannelError::AudioAction(None)))),
                Either::Right(_) => break Err(TaskError::Unrecoverable(UnrecoverableError::Stream)),
            }
        }
    }
}
