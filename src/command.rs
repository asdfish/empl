use {
    crossterm::Command,
    std::{fmt, io},
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

pub struct PrintBounded<T>(pub T, pub usize)
where
    T: AsChars;
impl<T> Command for PrintBounded<T>
where
    T: AsChars,
{
    fn write_ansi(&self, w: &mut impl fmt::Write) -> Result<(), fmt::Error> {
        self.0
            .as_chars()
            .try_fold(0, |mut width, ch| {
                width += ch.width().unwrap_or_default();

                if width > self.1 {
                    Err(TryFoldError::Break)
                } else if let Err(err) = w.write_char(ch) {
                    Err(TryFoldError::Fmt(err))
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
pub enum TryFoldError {
    Break,
    Fmt(fmt::Error),
}
impl From<fmt::Error> for TryFoldError {
    fn from(err: fmt::Error) -> TryFoldError {
        Self::Fmt(err)
    }
}
impl From<TryFoldError> for Result<(), fmt::Error> {
    fn from(err: TryFoldError) -> Result<(), fmt::Error> {
        match err {
            TryFoldError::Break => Ok(()),
            TryFoldError::Fmt(err) => Err(err),
        }
    }
}
