use {
    crate::config::{Config, SelectedConfig},
    arrayvec::ArrayVec,
    crossterm::event::{Event, EventStream, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    futures_core::Stream,
    std::{future::poll_fn, pin::Pin},
};

#[derive(Debug, Default)]
pub struct EventTask {
    key_presses: ArrayVec<(KeyModifiers, KeyCode), { SelectedConfig::MAX_KEY_BINDING_LEN.get() }>,
    stream: EventStream,
}
impl EventTask {
    pub async fn run(mut self) {
        loop {
            if let Some(Ok(Event::Key(KeyEvent {
                code,
                modifiers,
                kind: KeyEventKind::Press,
                ..
            }))) = poll_fn(|ctx| Pin::new(&mut self.stream).poll_next(ctx)).await
            {
                if let Err(_) = self.key_presses.try_push((modifiers, code)) {
                    self.key_presses.clear();
                    continue;
                }
            }
        }
    }
}
