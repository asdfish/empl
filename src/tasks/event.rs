use {
    crate::{
        config::{Config, KeyAction, SelectedConfig},
        ext::iterator::IteratorExt,
    },
    arrayvec::ArrayVec,
    crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    futures_core::Stream,
    std::{cmp::Ordering, future::poll_fn, pin::Pin},
    tokio::sync::mpsc,
};

#[derive(Debug)]
pub struct EventTask {
    pub action_tx: mpsc::UnboundedSender<KeyAction>,
    key_presses: ArrayVec<(KeyModifiers, KeyCode), { SelectedConfig::MAX_KEY_BINDING_LEN.get() }>,
    stream: EventStream,
}
impl EventTask {
    pub async fn run(&mut self) -> Result<(), mpsc::error::SendError<KeyAction>> {
        loop {
            let Some(Ok(Event::Key(KeyEvent {
                code,
                modifiers,
                kind: KeyEventKind::Press,
                ..
            }))) = poll_fn(|ctx| Pin::new(&mut self.stream).poll_next(ctx)).await
            else {
                continue;
            };

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
                        self.action_tx.send(*action)?;
                        self.key_presses.clear();
                    },
                    Some((_, Some(Ordering::Less))) => {}
                    _ => self.key_presses.clear(),
                }
        }
    }
}
