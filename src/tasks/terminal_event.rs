use {
    crate::{
        config::{Config, SelectedConfig},
        ext::iterator::IteratorExt,
        tasks::{ChannelError, display::state::Area, state},
    },
    arrayvec::ArrayVec,
    crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    futures_core::Stream,
    std::{cmp::Ordering, future::poll_fn, marker::PhantomData, num::NonZeroU16, pin::Pin},
    tokio::sync::mpsc,
};

#[derive(Debug)]
pub struct TerminalEventTask<'a> {
    pub event_tx: mpsc::UnboundedSender<state::Event>,
    key_presses: ArrayVec<(KeyModifiers, KeyCode), { SelectedConfig::MAX_KEY_BINDING_LEN.get() }>,
    stream: EventStream,
    _marker: PhantomData<&'a ()>,
}
impl<'a> TerminalEventTask<'a> {
    pub fn new(event_tx: mpsc::UnboundedSender<state::Event>) -> Self {
        Self {
            event_tx,
            key_presses: ArrayVec::new(),
            stream: EventStream::new(),
            _marker: PhantomData,
        }
    }

    pub async fn run(&mut self) -> Result<(), ChannelError<'a>> {
        loop {
            match poll_fn(|ctx| Pin::new(&mut self.stream).poll_next(ctx)).await {
                Some(Ok(Event::Key(KeyEvent {
                    code,
                    modifiers,
                    kind: KeyEventKind::Press,
                    ..
                }))) => {
                    if self.key_presses.try_push((modifiers, code)).is_err() {
                        self.key_presses.clear();
                        continue;
                    }

                    match SelectedConfig::KEY_BINDINGS
                        .iter()
                        .map(|(action, key_binding)| {
                            (action, self.key_presses.iter().containment(*key_binding))
                        })
                        .filter(|(_, ord)| *ord < Some(Ordering::Greater))
                        .max_by(|(_, l), (_, r)| l.cmp(r))
                    {
                        Some((action, Some(Ordering::Equal))) => {
                            self.event_tx.send(state::Event::KeyBinding(*action))?;
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
                            .send(state::Event::Resize(Area { width, height }))?;
                    }
                }
                _ => continue,
            }
        }
    }
}
