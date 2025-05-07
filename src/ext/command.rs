use {
    bumpalo::{collections::String as BString, Bump},
    crossterm::Command,
    std::{future::Future, io, marker::Unpin},
    tokio::io::AsyncWriteExt,
};

pub trait CommandExt: Sized {
    fn execute<W>(&self, _: &Bump, _: &mut W) -> impl Future<Output = Result<(), io::Error>>
    where
        W: AsyncWriteExt + Unpin;

    fn then<R>(self, r: R) -> Then<Self, R>
    where
        R: CommandExt,
    {
        Then { l: self, r }
    }
}
impl<T> CommandExt for T
where
    T: Command,
{
    fn execute<W>(&self, alloc: &Bump, out: &mut W) -> impl Future<Output = Result<(), io::Error>>
    where
        W: AsyncWriteExt + Unpin,
    {
        async move {
            #[cfg(windows)]
            if !self.is_ansi_code_supported() {
                return self.execute_winapi();
            }

            let mut buf = BString::new_in(alloc);
            let _ = self.write_ansi(&mut buf);

            out.write(buf.as_bytes()).await?;
            out.flush().await
        }
    }
}

pub struct Then<L, R>
where
    L: CommandExt,
    R: CommandExt,
{
    l: L,
    r: R,
}
impl<L, R> CommandExt for Then<L, R>
where
    L: CommandExt,
    R: CommandExt,
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
