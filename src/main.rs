#![cfg_attr(not(test), no_main)]

use {
    async_executor::LocalExecutor,
    async_io::block_on,
    empl::{
        argv::{ArgError, Argv, ArgvError},
        flag::{Arguments, ArgumentsError, Flag},
        // config::{Config, ConfigError},
    },
    std::{
        error::Error,
        ffi::{c_char, c_int},
        fmt::{self, Display, Formatter},
    },
};

/// Not implemented as `concat!("empl ", env!("CARGO_PKG_VERSION"))` to allow compiling without cargo.
const VERSION_MESSAGE: &str = "empl 0.1.0";

#[cfg_attr(not(test), unsafe(no_mangle))]
extern "system" fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    match (move || -> Result<(), MainError> {
        let mut flags = Arguments::new(unsafe { Argv::new(argc, argv) }?.skip(1));
        while let Some(flag) = flags.next().transpose()? {
            match flag {
                Flag::Short('h') | Flag::Long("help") => {
                    eprintln!(
                        "empl [OPTIONS..]

Most configuration is done at compile time by editing source code.
There are not any runtime configuration.

Options:
  -h --help    Print this message and exit.
  -v --version Print version information and exit."
                    );
                    return Ok(());
                }
                Flag::Short('v') | Flag::Long("version") => {
                    eprintln!("{}", VERSION_MESSAGE);
                    return Ok(());
                }
                flag => return Err(MainError::UnknownFlag(flag)),
            }
        }

        let executor = LocalExecutor::new();
        block_on(executor.run(async {
            // use {
            //     bumpalo::Bump,
            //     crossterm::style::{Color, Print, SetForegroundColor, ResetColor},
            //     empl::ext::command::CommandExt,
            //     std::io::stdout,
            // };

            // let b = Bump::new();
            // let _ = SetForegroundColor(Color::Red)
            //     .then(Print("foo"))
            //     .then(ResetColor)
            //     .execute(&b, &mut stdout().lock()).await;
        }));

        Ok(())
    })() {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("{}", err);
            1
        }
    }
}

#[derive(Debug)]
pub enum MainError {
    Arguments(ArgumentsError<'static, ArgError>),
    Argv(ArgvError),
    UnknownFlag(Flag<'static>),
}
impl Display for MainError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Arguments(e) => e.fmt(f),
            Self::Argv(e) => e.fmt(f),
            Self::UnknownFlag(flag) => write!(f, "unknown flag `{}`", flag),
        }
    }
}
impl Error for MainError {}
impl From<ArgumentsError<'static, ArgError>> for MainError {
    fn from(err: ArgumentsError<'static, ArgError>) -> Self {
        Self::Arguments(err)
    }
}
impl From<ArgvError> for MainError {
    fn from(err: ArgvError) -> Self {
        Self::Argv(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_sync() {
        if let Some(version) = option_env!("CARGO_PKG_VERSION") {
            assert_eq!(version, VERSION_MESSAGE.split_once(' ').unwrap().1);
        }
    }
}
