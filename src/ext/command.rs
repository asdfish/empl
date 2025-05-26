use {
    bumpalo::{Bump, collections::String as BString},
    crossterm::Command,
    std::{fmt, io, iter, marker::Unpin},
    tokio::io::AsyncWriteExt,
    unicode_width::UnicodeWidthChar,
};

pub trait CommandExt: Command + Sized {
    /// Convert a [Command] to something that implements [CommandChain]
    ///
    /// # Examples
    ///
    /// ```compile_fail
    /// # use empl::ext::command::CommandChain;
    /// # use crossterm::cursor;
    /// fn take_cmd_chain<C>(_: C)
    /// where C: CommandChain {}
    ///
    /// take_cmd_chain(cursor::Show);
    /// ```
    ///
    /// ```
    /// # use empl::ext::command::{CommandChain, CommandExt};
    /// # use crossterm::cursor;
    /// fn take_cmd_chain<C>(_: C)
    /// where C: CommandChain {}
    ///
    /// take_cmd_chain(cursor::Show.adapt());
    fn adapt(self) -> Adapter<Self> {
        Adapter(self)
    }
}
impl<T> CommandExt for T where T: Command {}

pub trait CommandChain: Sized {
    /// Execute the current command chain.
    ///
    /// Does not flush the buffer.
    fn execute<W>(self, _: &Bump, _: &mut W) -> impl Future<Output = Result<(), io::Error>>
    where
        W: AsyncWriteExt + Unpin;

    /// Chain two commands together sequentially.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use empl::ext::command::{CommandChain, CommandExt};
    /// # use bumpalo::Bump;
    /// # use crossterm::cursor;
    /// # use tokio::io::stdout;
    /// # async {
    /// cursor::Show.adapt()
    ///     .then(cursor::MoveTo(0, 0).adapt())
    ///     .execute(&Bump::new(), &mut stdout()).await?;
    /// # Ok::<(), std::io::Error>(())
    /// # };
    /// ```
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
            out.flush().await?;
            return self.0.execute_winapi();
        }

        let mut buf = BString::new_in(alloc);
        let _ = self.0.write_ansi(&mut buf);

        out.write_all(buf.as_bytes()).await.map(drop)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CommandIter<I, T>(pub I)
where
    I: Iterator<Item = T>,
    T: CommandChain;
impl<I, T> CommandChain for CommandIter<I, T>
where
    I: Iterator<Item = T>,
    T: CommandChain,
{
    async fn execute<W>(self, alloc: &Bump, out: &mut W) -> Result<(), io::Error>
    where
        W: AsyncWriteExt + Unpin,
    {
        for cmd in self.0 {
            cmd.execute(alloc, out).await?;
        }

        Ok(())
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

pub trait AsChars {
    fn as_chars(&self) -> impl Iterator<Item = char>;
}
impl<T> AsChars for T
where
    T: AsRef<str>,
{
    fn as_chars(&self) -> impl Iterator<Item = char> {
        self.as_ref().chars()
    }
}

pub struct PrintPadded<T>
where
    T: AsChars,
{
    pub text: T,
    pub padding: char,
    pub width: usize,
}
impl<T> Command for PrintPadded<T>
where
    T: AsChars,
{
    fn write_ansi(&self, w: &mut impl fmt::Write) -> Result<(), fmt::Error> {
        self.text
            .as_chars()
            .chain(iter::repeat(self.padding))
            .try_fold(0, |mut width, ch| {
                width += ch.width().unwrap_or_default();

                if width > self.width {
                    Err(TryFoldShortCircuit::Break)
                } else if let Err(err) = w.write_char(ch) {
                    Err(TryFoldShortCircuit::Fmt(err))
                } else {
                    Ok(width)
                }
            })
            .map(drop)
            .or_else(Result::<(), fmt::Error>::from)
    }

    #[cfg(windows)]
    fn is_ansi_code_supported(&self) -> bool {
        true
    }

    #[cfg(windows)]
    fn execute_winapi(&self) -> Result<(), std::io::Error> {
        Ok(())
    }
}

#[derive(Debug)]
pub enum TryFoldShortCircuit {
    Break,
    Fmt(fmt::Error),
}
impl From<fmt::Error> for TryFoldShortCircuit {
    fn from(err: fmt::Error) -> TryFoldShortCircuit {
        Self::Fmt(err)
    }
}
impl From<TryFoldShortCircuit> for Result<(), fmt::Error> {
    fn from(err: TryFoldShortCircuit) -> Result<(), fmt::Error> {
        match err {
            TryFoldShortCircuit::Break => Ok(()),
            TryFoldShortCircuit::Fmt(err) => Err(err),
        }
    }
}
