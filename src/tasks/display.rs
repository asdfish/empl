use {
    crate::{
        config::Playlists,
        display::state::{DisplayState, DisplayStateWriter},
        ext::command::CommandChain,
    },
    bumpalo::Bump,
    std::io,
    tokio::{
        io::{stdout, Stdout},
        sync::mpsc,
    },
};

#[derive(Debug)]
pub struct DisplayTask<'a> {
    alloc: Bump,
    state: DisplayStateWriter<'a>,
    stdout: Stdout,
    rx: mpsc::Receiver<&'static dyn for<'b> Fn(DisplayState<'b>) -> DisplayState<'b>>,
}
impl<'a> DisplayTask<'a> {
    pub fn new(playlists: &'a Playlists, rx: mpsc::Receiver<&'static dyn for<'b> Fn(DisplayState<'b>) -> DisplayState<'b>>) -> Self {
        Self {
            alloc: Bump::new(),
            state: DisplayStateWriter::new(playlists),
            stdout: stdout(),
            rx,
        }
    }

    pub async fn run(mut self) -> Result<(), io::Error> {
        while let Some(action) = self.rx.recv().await {
            self.alloc.reset();
            let (damages, old_state) = self.state.write(action);

            for damage in damages {
                damage.render(&old_state, self.state.as_ref())
                    .execute(&self.alloc, &mut self.stdout).await?;
            }
        }

        Ok(())
    }
}
