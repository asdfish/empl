// empl - Extensible Music PLayer
// Copyright (C) 2025  Andrew Chi

// This file is part of empl.

// empl is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// empl is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with empl.  If not, see <http://www.gnu.org/licenses/>.

//! Parser for the config created from command line options.

use {
    crate::{
        config::{default_paths::DEFAULT_PATHS, path_segments::choice::Choice},
        display::IntoDisplay,
    },
    const_format::{formatc, formatcp},
    getargs::{Opt, Options},
    std::{
        error::Error,
        fmt::{self, Display, Formatter},
        io::{self, Write, stdout},
        path::Path,
    },
};

unsafe extern "C" {
    /// # Safety
    ///
    /// This function is call to safe if it does not exist.
    fn _link_error() -> !;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Config<'a> {
    /// The path to the entry point for the configuration file.
    _config_file_entry: Option<&'a Path>,
}
impl<'a> Config<'a> {
    /// Parser some cli flags.
    ///
    /// # Output
    ///
    /// - `Ok(Some(_))` indicates that the config was sucessfully parsed.
    /// - `Ok(None)` indicates the user selected an option that stopped parsing successfully such as `-h` or `-v`.
    /// - `Err(_)` indicates an error during parsing.
    pub fn new<I>(iter: I) -> Result<Option<Self>, ParseCliArgumentsError<'a>>
    where
        I: IntoIterator<Item = &'a [u8]>,
    {
        let output = Self::default();
        let mut opts = Options::new(iter.into_iter());

        #[expect(clippy::never_loop)]
        while let Some(opt) = opts.next_opt()? {
            match opt {
                Opt::Short(b'h') | Opt::Long(b"help") => {
                    let mut stdout = stdout().lock();
                    return stdout
                        .write_all(
                            const {
                                formatc!(
                                    "Usage: {} [OPTIONS..]

Options:
  -h --help           Print this message and exit.
  -v --version        Print version information and exit.
  -c --config  [PATH] Set the path to the entrypoint to the config file.
                      Defaults to {}.\n",
                                    env!("CARGO_BIN_NAME"),
                                    Choice::new(DEFAULT_PATHS).unwrap(),
                                )
                                .as_bytes()
                            },
                        )
                        .and_then(|_| stdout.flush())
                        .map(|_| None)
                        .map_err(ParseCliArgumentsError::PrintStdout);
                }
                Opt::Short(b'v') | Opt::Long(b"version") => {
                    let mut stdout = stdout().lock();
                    return stdout
                        .write_all(
                            const {
                                formatcp!(
                                    "{} {}\n",
                                    env!("CARGO_BIN_NAME"),
                                    env!("CARGO_PKG_VERSION")
                                )
                                .as_bytes()
                            },
                        )
                        .and_then(|_| stdout.flush())
                        .map(|_| None)
                        .map_err(ParseCliArgumentsError::PrintStdout);
                }
                flag => return Err(ParseCliArgumentsError::UnknownFlag(flag)),
            }
        }

        Ok(Some(output))
    }
}

#[derive(Debug)]
pub enum ParseCliArgumentsError<'a> {
    PrintStdout(io::Error),
    MissingValue(Opt<&'a [u8]>),
    UnexpectedValue(Opt<&'a [u8]>),
    UnknownFlag(Opt<&'a [u8]>),
}
impl Display for ParseCliArgumentsError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::PrintStdout(error) => write!(f, "failed to write to stdout: {error}"),
            Self::MissingValue(flag) => write!(f, "flag `{}` requires a value", flag.display()),
            Self::UnexpectedValue(flag) => {
                write!(f, "flag `{}` does not take a value", flag.display())
            }
            Self::UnknownFlag(flag) => write!(f, "unexpected flag `{}`", flag.display()),
        }
    }
}
impl Error for ParseCliArgumentsError<'_> {}
impl<'a> From<getargs::Error<&'a [u8]>> for ParseCliArgumentsError<'a> {
    fn from(error: getargs::Error<&'a [u8]>) -> Self {
        match error {
            getargs::Error::RequiresValue(flag) => ParseCliArgumentsError::MissingValue(flag),
            getargs::Error::DoesNotRequireValue(flag) => {
                ParseCliArgumentsError::UnexpectedValue(flag)
            }
            _ => {
                // SAFETY: this will cause a link error.
                unsafe { _link_error() }
            }
        }
    }
}
