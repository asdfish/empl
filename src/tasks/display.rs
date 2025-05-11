pub mod damage;
pub mod state;

use {
    crate::{
        ext::command::CommandChain,
        tasks::{ChannelError, display::damage::DamageList},
    },
    bumpalo::Bump,
    tokio::{
        io::{AsyncWriteExt, Stdout},
        sync::mpsc,
    },
};

#[derive(Debug)]
pub struct DisplayTask<'a> {
    alloc: Bump,
    stdout: Stdout,
    pub display_rx: mpsc::Receiver<DamageList<'a>>,
}
impl<'a> DisplayTask<'a> {
    pub fn new(alloc: Bump, stdout: Stdout, display_rx: mpsc::Receiver<DamageList<'a>>) -> Self {
        Self {
            alloc,
            stdout,
            display_rx,
        }
    }

    pub async fn run(&mut self) -> Result<(), ChannelError<'a>> {
        loop {
            let action = self
                .display_rx
                .recv()
                .await
                .ok_or(ChannelError::Display(None))?;
            self.alloc.reset();
            let _ = action.execute(&self.alloc, &mut self.stdout).await;
            let _ = self.stdout.flush().await;
        }
    }
}
