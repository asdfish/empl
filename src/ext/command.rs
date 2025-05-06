use {
    async_io::{Async, IoSafe},
    bumpalo::{collections::String as BString, Bump},
    crossterm::Command,
    futures_lite::AsyncWriteExt,
    std::{fmt, future::Future, io, os::fd::AsFd},
};

pub trait CommandExt: Command + Sized {
    fn execute<W>(&self, b: &Bump, out: W) -> impl Future<Output = Result<(), io::Error>>
    where
        W: AsFd + IoSafe + io::Write,
    {
        async {
            #[cfg(windows)]
            if !self.is_ansi_code_supported() {
                return self.execute_winapi();
            }

            let mut buf = BString::new_in(b);
            let _ = self.write_ansi(&mut buf);

            let mut adapter = Async::new(out)?;
            adapter.write(buf.as_bytes()).await?;
            adapter.flush().await
        }
    }

    fn then<R>(self, r: R) -> Then<Self, R>
    where
        R: Command,
    {
        Then { l: self, r }
    }
}
impl<T> CommandExt for T where T: Command {}

#[derive(Clone, Copy, Debug)]
pub struct Then<L, R>
where
    L: Command,
    R: Command,
{
    l: L,
    r: R,
}
impl<L, R> Command for Then<L, R>
where
    L: Command,
    R: Command,
{
    fn write_ansi(&self, f: &mut impl fmt::Write) -> Result<(), fmt::Error> {
        self.l.write_ansi(f)?;
        self.r.write_ansi(f)
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> Result<(), io::Error> {
        self.l.execute_winapi()?;
        self.r.execute_winapi()
    }

    #[cfg(windows)]
    fn is_ansi_code_supported(&self) -> bool {
        self.l.is_ansi_code_supported() && self.r.is_ansi_code_supported()
    }
}
