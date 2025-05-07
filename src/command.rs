use {
    crossterm::Command,
    std::{fmt, io, iter},
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
