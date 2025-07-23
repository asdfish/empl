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
    cfg_if::cfg_if,
    const_format::formatcp,
    getargs::{Opt, Options},
    std::{
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

const fn human_default_config_dirs() -> &'static str {
    cfg_if! {
        if #[cfg(windows)] {
            return r#"`${%APPDATA%}\empl\config\main.scm`"#;
        } else if #[cfg(target_os = "macos")] {
            return "`${HOME}/Library/Application Support/empl/main.scm`";
        } else if #[cfg(unix)] {
            return "`${XDG_CONFIG_HOME}/empl/main.scm` or `${HOME}/.config/empl/main.scm`";
        } else {
            compile_error!("unsupported platform");
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Config<'a> {
    /// The path to the entry point for the configuration file.
    config_file_entry: Option<&'a Path>,
}
impl<'a> Config<'a> {
    /// Parser some cli flags.
    ///
    /// # Output
    ///
    /// - `Ok(Some(_))` indicates that the config was sucessfully parsed.
    /// - `Ok(None)` indicates the user selected an option that stopped parsing successfully such as `-h` or `-v`.
    /// - `Err(_)` indicates an error during parsing.
    pub fn new<I>(iter: I) -> Result<Option<Self>, ParserCliArgumentsError<'a>>
    where
        I: IntoIterator<Item = &'a [u8]>,
    {
        let mut opts = Options::new(iter.into_iter());

        while let Some(opt) = opts.next_opt()? {
            match opt {
                Opt::Short(b'h') | Opt::Long(b"help") => {
                    let mut stdout = stdout().lock();
                    return stdout
                        .write_all(
                            const {
                                formatcp!(
                                    "Usage: {} [OPTIONS..]

Options:
  -h --help           Print this message and exit.
  -v --version        Print version information and exit.
  -c --config  [PATH] Set the path to the entrypoint to the config file.
                      Defaults to {}.",
                                    env!("CARGO_BIN_NAME"),
                                    human_default_config_dirs(),
                                )
                                .as_bytes()
                            },
                        )
                        .and_then(|_| stdout.flush())
                        .map(|_| None)
                        .map_err(ParserCliArgumentsError::PrintStdout);
                }
                Opt::Short(b'v') | Opt::Long(b"version") => {
                    let mut stdout = stdout().lock();
                    return stdout
                        .write_all(
                            const {
                                formatcp!(
                                    "{} {}",
                                    env!("CARGO_BIN_NAME"),
                                    env!("CARGO_PKG_VERSION")
                                )
                                .as_bytes()
                            },
                        )
                        .and_then(|_| stdout.flush())
                        .map(|_| None)
                        .map_err(ParserCliArgumentsError::PrintStdout);
                }
                flag => return Err(ParserCliArgumentsError::UnknownFlag(flag)),
            }
        }

        todo!()
    }
}

pub enum ParserCliArgumentsError<'a> {
    PrintStdout(io::Error),
    MissingValue(Opt<&'a [u8]>),
    UnexpectedValue(Opt<&'a [u8]>),
    UnknownFlag(Opt<&'a [u8]>),
}
impl<'a> From<getargs::Error<&'a [u8]>> for ParserCliArgumentsError<'a> {
    fn from(error: getargs::Error<&'a [u8]>) -> Self {
        match error {
            getargs::Error::RequiresValue(flag) => ParserCliArgumentsError::MissingValue(flag),
            getargs::Error::DoesNotRequireValue(flag) => {
                ParserCliArgumentsError::UnexpectedValue(flag)
            }
            _ => {
                // SAFETY: this will cause a link error.
                unsafe { _link_error() }
            }
        }
    }
}
