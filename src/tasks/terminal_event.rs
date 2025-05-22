use {
    crate::{
        config::Config,
        ext::iterator::IteratorExt,
        tasks::{ChannelError, display::state::Area, state},
    },
    crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    futures_core::Stream,
    nonempty_collections::iter::NonEmptyIterator,
    std::{
        cmp::{Ordering, max},
        future::poll_fn,
        num::{NonZeroU16, NonZeroUsize},
        pin::Pin,
    },
    tokio::sync::mpsc,
};

#[derive(Debug)]
pub struct TerminalEventTask<'a> {
    config: &'a Config,
    max_key_binding_len: Option<NonZeroUsize>,

    pub event_tx: mpsc::Sender<state::Event>,
    key_presses: Vec<(KeyModifiers, KeyCode)>,
    stream: EventStream,
}
impl<'a> TerminalEventTask<'a> {
    pub fn new(config: &'a Config, event_tx: mpsc::Sender<state::Event>) -> Self {
        Self {
            config,
            max_key_binding_len: None,

            event_tx,
            key_presses: Vec::new(),
            stream: EventStream::new(),
        }
    }

    pub async fn run<'b>(&mut self) -> Result<(), ChannelError<'b>> {
        loop {
            match poll_fn(|ctx| Pin::new(&mut self.stream).poll_next(ctx)).await {
                Some(Ok(Event::Key(KeyEvent {
                    code,
                    modifiers,
                    kind: KeyEventKind::Press,
                    ..
                }))) => {
                    self.key_presses.push((modifiers, code));
                    if self.key_presses.len()
                        >= self
                            .max_key_binding_len
                            .get_or_insert_with(|| {
                                self.config
                                    .key_bindings
                                    .nonempty_iter()
                                    .map(|(_, key_binding)| key_binding.len())
                                    .reduce(max)
                            })
                            .get()
                    {
                        self.key_presses.clear();
                        continue;
                    }

                    match self
                        .config
                        .key_bindings
                        .iter()
                        .map(|(action, key_binding)| {
                            (action, self.key_presses.iter().containment(key_binding))
                        })
                        .filter(|(_, ord)| *ord < Some(Ordering::Greater))
                        .max_by(|(_, l), (_, r)| l.cmp(r))
                    {
                        Some((action, Some(Ordering::Equal))) => {
                            self.event_tx
                                .send(state::Event::KeyBinding(*action))
                                .await?;
                            self.key_presses.clear();
                        }
                        Some((_, Some(Ordering::Less))) => {}
                        _ => self.key_presses.clear(),
                    }
                }
                Some(Ok(Event::Resize(width, height))) => {
                    if let (Some(width), Some(height)) =
                        (NonZeroU16::new(width), NonZeroU16::new(height))
                    {
                        self.event_tx
                            .send(state::Event::Resize(Area { width, height }))
                            .await?;
                    }
                }
                _ => continue,
            }
        }
    }
}
