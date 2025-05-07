use {
    bumpalo::{Bump, collections::String as BString},
    crossterm::Command,
    either::Either,
    std::{future::Future, io, marker::Unpin},
    tokio::io::AsyncWriteExt,
};

pub trait CommandExt: Command + Sized {
    fn adapt(self) -> Adapter<Self> {
        Adapter(self)
    }
}
impl<T> CommandExt for T where T: Command {}

pub trait CommandChain: Sized {
    fn execute<W>(self, _: &Bump, _: &mut W) -> impl Future<Output = Result<(), io::Error>>
    where
        W: AsyncWriteExt + Unpin;

    fn then<R>(self, r: R) -> Then<Self, R>
    where
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
    async fn execute<W>(self, alloc: &Bump, out: &mut W) -> Result<(), io::Error>
    where
        W: AsyncWriteExt + Unpin,
    {
        #[cfg(windows)]
        if !self.0.is_ansi_code_supported() {
            return self.0.execute_winapi();
        }

        let mut buf = BString::new_in(alloc);
        let _ = self.0.write_ansi(&mut buf);

        out.write_all(buf.as_bytes()).await?;
        out.flush().await
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CommandIter<I, T, M, C>(pub(super) I, pub(super) M)
where
    I: Iterator<Item = T>,
    M: FnMut(T) -> C,
    C: CommandChain;
impl<I, T, M, C> CommandChain for CommandIter<I, T, M, C>
where
    I: Iterator<Item = T>,
    M: FnMut(T) -> C,
    C: CommandChain,
{
    async fn execute<W>(mut self, alloc: &Bump, out: &mut W) -> Result<(), io::Error>
    where
        W: AsyncWriteExt + Unpin,
    {
        while let Some(cmd) = self.0.next().map(&mut self.1) {
            cmd.execute(alloc, out).await?;
        }

        Ok(())
    }
}

impl<L, R> CommandChain for Either<L, R>
where
    L: CommandChain,
    R: CommandChain,
{
    async fn execute<W>(self, alloc: &Bump, out: &mut W) -> Result<(), io::Error>
    where
        W: AsyncWriteExt + Unpin,
    {
        match self {
            Self::Left(cmd) => cmd.execute(alloc, out).await,
            Self::Right(cmd) => cmd.execute(alloc, out).await,
        }
    }
}

impl<T> CommandChain for Option<T>
where
    T: CommandChain,
{
    async fn execute<W>(self, alloc: &Bump, out: &mut W) -> Result<(), io::Error>
    where
        W: AsyncWriteExt + Unpin,
    {
        match self {
            Some(cmd) => cmd.execute(alloc, out).await,
            None => Ok(()),
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
    async fn execute<W>(self, alloc: &Bump, out: &mut W) -> Result<(), io::Error>
    where
        W: AsyncWriteExt + Unpin,
    {
        self.l.execute(alloc, out).await?;
        self.r.execute(alloc, out).await
    }
}
