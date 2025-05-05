//! Terminal commands

pub mod color;

use {
    bumpalo::{
        Bump,
        collections::String as BString,
    },
};

pub trait Command {
    #[cfg(windows)]
    const WORKS_ON_WINDOWS: bool;

    /// Return ansi characters as string.
    fn ansi<'a>(&self, _: &'a Bump) -> BString<'a>;

    #[cfg(windows)]
    fn execute_windows();
}
