use {
    bumpalo::{collections::String as BString, Bump},
    crossterm::Command,
    std::{future::Future, io, marker::Unpin},
    tokio::io::AsyncWriteExt,
};

pub trait CommandExt: Command + Sized {
    fn adapt(self) -> Adapter<Self> {
        Adapter(self)
    }
}
impl<T> CommandExt for T
where T: Command {}

pub trait CommandChain {
    fn execute<W>(&self, _: &Bump, _: &mut W) -> impl Future<Output = Result<(), io::Error>>
    where
        W: AsyncWriteExt + Unpin;

    fn then<R>(self, r: R) -> Then<Self, R>
    where
        Self: Sized,
        R: CommandChain,
    {
        Then { l: self, r }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Adapter<C>(C)
where
    C: Command;
impl<C> CommandChain for Adapter<C>
where
    C: Command,
{
    fn execute<W>(&self, alloc: &Bump, out: &mut W) -> impl Future<Output = Result<(), io::Error>>
    where
        W: AsyncWriteExt + Unpin,
    {
        async move {
            #[cfg(windows)]
            if !self.is_ansi_code_supported() {
                return self.0.execute_winapi();
            }

            let mut buf = BString::new_in(alloc);
            let _ = self.0.write_ansi(&mut buf);

            out.write_all(buf.as_bytes()).await?;
            out.flush().await
        }
    }
}

impl<T> CommandChain for Option<T>
where
    T: CommandChain,
{
    fn execute<W>(&self, alloc: &Bump, out: &mut W) -> impl Future<Output = Result<(), io::Error>>
    where
        W: AsyncWriteExt + Unpin,
    {
        async move {
            match self {
                Some(cmd) => cmd.execute(alloc, out).await,
                None => Ok(()),
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Then<L, R>
where
    L: CommandChain,
    R: CommandChain,
{
    l: L,
    r: R,
}
impl<L, R> CommandChain for Then<L, R>
where
    L: CommandChain,
    R: CommandChain,
{
    fn execute<W>(&self, alloc: &Bump, out: &mut W) -> impl Future<Output = Result<(), io::Error>>
    where
        W: AsyncWriteExt + Unpin,
    {
        async {
            self.l.execute(alloc, out).await?;
            self.r.execute(alloc, out).await
        }
    }
}
