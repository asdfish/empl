pub mod damage;
pub mod state;

use {
    crate::{
        ext::command::{CommandChain, CommandExt},
        tasks::{ChannelError, display::damage::DamageList},
    },
    bumpalo::Bump,
    crossterm::{
        QueueableCommand, cursor,
        terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
    },
    std::io::{self, Write},
    tokio::{
        io::{AsyncWriteExt, Stdout, stdout},
        sync::mpsc,
    },
};

#[derive(Debug)]
pub struct DisplayTask<'a> {
    alloc: Bump,
    stdout: Stdout,
    pub display_rx: mpsc::UnboundedReceiver<DamageList<'a>>,
}
impl<'a> DisplayTask<'a> {
    pub async fn new(
        display_rx: mpsc::UnboundedReceiver<DamageList<'a>>,
    ) -> Result<Self, io::Error> {
        let alloc = Bump::new();
        let mut stdout = stdout();

        enable_raw_mode()?;
        cursor::Hide
            .adapt()
            .then(EnterAlternateScreen.adapt())
            .execute(&alloc, &mut stdout)
            .await?;
        stdout.flush().await?;

        Ok(Self {
            alloc,
            stdout,
            display_rx,
        })
    }

    pub async fn run(&mut self) -> Result<(), ChannelError<'a>> {
        loop {
            let action = self.display_rx.recv().await.ok_or(ChannelError::Display(None))?;
            self.alloc.reset();
            let _ = action.execute(&self.alloc, &mut self.stdout).await;
            let _ = self.stdout.flush().await;
        }
    }
}
impl Drop for DisplayTask<'_> {
    fn drop(&mut self) {
        let mut stdout = std::io::stdout();
        let _ = stdout.queue(LeaveAlternateScreen);
        let _ = stdout.queue(cursor::Show);
        let _ = stdout.flush();
        let _ = disable_raw_mode();
    }
}
