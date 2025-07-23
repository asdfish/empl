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
        ffi::OsStr,
        fmt::{self, Display, Formatter},
        io::{self, Write},
        path::Path,
    },
};

unsafe extern "C" {
    /// # Safety
    ///
    /// This function is call to safe if it does not exist.
    fn link_error() -> !;
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Config<'a> {
    /// The path to the entry point for the configuration file.
    config_file: Option<&'a Path>,
    exprs: Vec<&'a [u8]>,
}
impl<'a> Config<'a> {
    /// Parser some cli flags.
    ///
    /// # Output
    ///
    /// - `Ok(Some(_))` indicates that the config was sucessfully parsed.
    /// - `Ok(None)` indicates the user selected an option that stopped parsing successfully such as `-h` or `-v`.
    /// - `Err(_)` indicates an error during parsing.
    pub fn new<I, O>(iter: I, stdout: &mut O) -> Result<Option<Self>, ParseCliArgumentsError<'a>>
    where
        I: IntoIterator<Item = &'a [u8]>,
        O: Write,
    {
        let mut output = Self::default();
        let mut opts = Options::new(iter.into_iter());

        while let Some(opt) = opts.next_opt()? {
            match opt {
                Opt::Short(b'h') | Opt::Long(b"help") => {
                    return stdout
                        .write_all(
                            const {
                                formatc!(
                                    "Usage: {} [OPTIONS..]

Options:
  -h --help           Print this message and exit.
  -v --version        Print version information and exit.
  -c --config  [PATH] Set the path to the entrypoint to the config file.
                      Defaults to {}.
  -e --eval    [EXPR] Add an expression that will be evaluated at the end
                      of the config file.\n",
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
                Opt::Short(b'c') | Opt::Long(b"config") => {
                    output.config_file = Some(Path::new(unsafe {
                        OsStr::from_encoded_bytes_unchecked(opts.value()?)
                    }));
                }
                Opt::Short(b'e') | Opt::Long(b"eval") => {
                    output.exprs.push(opts.value()?);
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
                unsafe { link_error() }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        std::{io, iter},
    };

    #[test]
    fn cli_required_args() {
        [b"-c" as &[u8], b"--config", b"-e", b"--eval"]
            .into_iter()
            .for_each(|arg| {
                assert!(matches!(
                    Config::new(iter::once(arg), &mut io::empty()).unwrap_err(),
                    ParseCliArgumentsError::MissingValue(_)
                ))
            })
    }

    #[test]
    fn cli_output() {
        [
            (&[b"-h" as &[u8]] as &[&[u8]], None),
            (&[b"--help"], None),
            (&[b"-v"], None),
            (&[b"--version"], None),
            (
                &[b"-cfoo"],
                Some(Config {
                    config_file: Some(Path::new("foo")),
                    ..Default::default()
                }),
            ),
            (
                &[b"--config", b"foo"],
                Some(Config {
                    config_file: Some(Path::new("foo")),
                    ..Default::default()
                }),
            ),
            (
                &[b"-efoo", b"--eval", b"bar"],
                Some(Config {
                    exprs: vec![b"foo", b"bar"],
                    ..Default::default()
                }),
            ),
        ]
        .into_iter()
        .for_each(|(args, output)| {
            assert_eq!(
                Config::new(args.iter().copied(), &mut io::empty()).unwrap(),
                output
            )
        });
    }

    #[test]
    fn stdout_ends_in_newline() {
        let mut stdout = Vec::new();

        [b"-h" as &[u8], b"--help", b"-v", b"--version"]
            .into_iter()
            .for_each(|flag| {
                stdout.clear();
                Config::new(iter::once(flag), &mut stdout).unwrap();
                assert_eq!(*stdout.last().unwrap(), b'\n');
            })
    }
}
