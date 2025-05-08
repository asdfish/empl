use {
    crate::{
        display::damage::DamageList,
        ext::command::CommandChain,
    },
    bumpalo::Bump,
    std::io,
    tokio::{
        io::{Stdout, stdout},
        sync::mpsc,
    },
};

#[derive(Debug)]
pub struct DisplayTask<'a> {
    alloc: Bump,
    stdout: Stdout,
    rx: mpsc::Receiver<DamageList<'a>>,
}
impl<'a> DisplayTask<'a> {
    pub fn new(
        rx: mpsc::Receiver<DamageList<'a>>
    ) -> Self {
        Self {
            alloc: Bump::new(),
            stdout: stdout(),
            rx,
        }
    }

    pub async fn run(&mut self) -> Result<(), io::Error> {
        while let Some(action) = self.rx.recv().await {
            self.alloc.reset();
            action.execute(&self.alloc, &mut self.stdout).await?;
        }

        Ok(())
    }
}
