use {
    crate::{
        either::{Either, EitherOutput},
        tasks::{ChannelError, RecoverableError, TaskError, UnrecoverableError, CHANNEL_SIZE, state::Event},
    },
    awedio::{
        Sound,
        backends::{CpalBackend as Backend, CpalBackendError as AudioBackendError},
        manager::Manager,
        sounds,
    },
    std::{path::Path, sync::Arc},
    tokio::sync::{mpsc, oneshot},
};

#[derive(Debug)]
pub enum AudioAction {
    Play(Arc<Path>),
}

pub struct AudioTask {
    pub action_rx: mpsc::Receiver<AudioAction>,
    pub change_completion_notifier_tx: mpsc::Sender<oneshot::Receiver<()>>,
    pub event_tx: mpsc::Sender<Event>,
    pub error_rx: mpsc::Receiver<()>,
    manager: Manager,
    _backend: Backend,
}
impl AudioTask {
    pub fn new(
        action_rx: mpsc::Receiver<AudioAction>,
        change_completion_notifier_tx: mpsc::Sender<oneshot::Receiver<()>>,
        event_tx: mpsc::Sender<Event>,
    ) -> Result<Self, AudioBackendError> {
        let mut backend = Backend::with_defaults().ok_or(AudioBackendError::NoDevice)?;
        let (error_tx, error_rx) = mpsc::channel(CHANNEL_SIZE);
        let error_tx = error_tx;

        Ok(Self {
            action_rx,
            change_completion_notifier_tx,
            event_tx,
            error_rx,
            manager: backend.start(move |_| {
                let _ = error_tx.blocking_send(());
            })?,
            _backend: backend,
        })
    }

    async fn play<'a, P>(&mut self, path: &P) -> Result<(), ChannelError<'a>>
    where
        P: AsRef<Path> + ?Sized,
    {
        let (sound, completion_notifier) = match sounds::open_file(path) {
            Ok(s) => s,
            Err(_) => {
                return Ok(());
            }
        }
        .with_async_completion_notifier();
        self.change_completion_notifier_tx
            .send(completion_notifier)
            .await?;

        self.manager.play(Box::new(sound));
        Ok(())
    }

    pub async fn run<'a>(&mut self) -> Result<(), TaskError<'a>> {
        loop {
            match EitherOutput::new(self.action_rx.recv(), self.error_rx.recv()).await {
                Either::Left(Some(AudioAction::Play(path))) => {
                    self.manager.clear();
                    self.play(&path)
                        .await
                        .map_err(RecoverableError::Channel)
                        .map_err(TaskError::Recoverable)?;
                }
                Either::Left(None) => {
                    break Err(TaskError::Recoverable(RecoverableError::Channel(
                        ChannelError::AudioAction(None),
                    )));
                }
                Either::Right(_) => {
                    break Err(TaskError::Unrecoverable(UnrecoverableError::Stream));
                }
            }
        }
    }
}
