use {
    crossterm::Command,
    std::{fmt, iter},
    unicode_width::UnicodeWidthChar,
};

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
    fn execute_winapi(&self) -> Result<(), io::Error> {
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
