use {
    bumpalo::{
        collections::String as BString,
        Bump,
    },
    crate::command::Command,
    std::fmt::Write,
};

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
#[rustfmt::skip]
pub enum Color {
    Black = 0,
    DarkRed = 1,
    DarkGreen = 2,
    DarkYellow = 3,
    DarkBlue = 4,
    DarkPurple = 5,
    DarkCyan = 6,
    LightGray = 7,

    DarkGray = 8,
    Red = 9,
    Green = 10,
    Yellow = 11,
    Blue = 12,
    Purple = 13,
    Cyan = 14,
    White = 15,
}

#[derive(Clone, Copy, Debug)]
pub struct Foreground(pub Color);
impl Command for Foreground {
    #[cfg(windows)]
    const WORKS_ON_WINDOWS: bool = true;

    fn ansi<'a>(&self, a: &'a Bump) -> BString<'a> {
        let mut buf = BString::new_in(a);
        let _ = write!(buf, "\x1b[38;5;{}m", self.0 as u8);

        buf
    }

    #[cfg(windows)]
    fn execute_windows() {}
}

#[derive(Clone, Copy, Debug)]
pub struct Background(pub Color);
impl Command for Background {
    #[cfg(windows)]
    const WORKS_ON_WINDOWS: bool = true;

    fn ansi<'a>(&self, a: &'a Bump) -> BString<'a> {
        let mut buf = BString::new_in(a);
        let _ = write!(buf, "\x1b[48;5;{}m", self.0 as u8);

        buf
    }

    #[cfg(windows)]
    fn execute_windows() {}
}
