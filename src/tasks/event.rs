use {
    crate::{
        config::{Config, KeyAction, SelectedConfig},
        ext::iterator::IteratorExt,
    },
    arrayvec::ArrayVec,
    crossterm::event::{Event as TermEvent, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    futures_core::Stream,
    std::{cmp::Ordering, future::poll_fn, io, pin::Pin},
    tokio::sync::mpsc,
};

#[derive(Debug)]
pub struct EventTask {
    pub event_tx: mpsc::UnboundedSender<Event>,
    key_presses: ArrayVec<(KeyModifiers, KeyCode), { SelectedConfig::MAX_KEY_BINDING_LEN.get() }>,
    stream: EventStream,
}
impl EventTask {
    pub fn new(event_tx: mpsc::UnboundedSender<Event>) -> Self {
        Self {
            event_tx,
            key_presses: ArrayVec::new(),
            stream: EventStream::new(),
        }
    }

    pub async fn run(&mut self) -> Result<(), mpsc::error::SendError<Event>> {
        loop {
            match poll_fn(|ctx| Pin::new(&mut self.stream).poll_next(ctx)).await {
                Some(Ok(TermEvent::Key(KeyEvent {
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
                        .max_by(|(_, l), (_, r)| l.cmp(r)) {
                            Some((action, Some(Ordering::Equal))) => {
                                self.event_tx.send(Event::KeyBinding(*action))?;
                                self.key_presses.clear();
                            },
                            Some((_, Some(Ordering::Less))) => {}
                            _ => self.key_presses.clear(),
                        }
                }
                _ => continue,
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Event {
    KeyBinding(KeyAction),
}
