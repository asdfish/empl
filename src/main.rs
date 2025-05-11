#![cfg_attr(not(test), no_main)]

use {
    empl::{
        argv::{ArgError, Argv, ArgvError},
        config::{Config, SelectedConfig},
        flag::{Arguments, ArgumentsError, Flag},
        tasks::{NewTaskManagerError, TaskManager, UnrecoverableError},
    },
    std::{
        error::Error,
        ffi::{c_char, c_int},
        fmt::{self, Display, Formatter},
        io,
    },
    tokio::runtime,
};

/// Not implemented as `concat!("empl ", env!("CARGO_PKG_VERSION"))` to allow compiling without cargo.
const VERSION_MESSAGE: &str = "empl 0.1.0";

#[cfg_attr(not(test), unsafe(no_mangle))]
extern "system" fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    match (move || -> Result<(), MainError> {
        let mut flags = Arguments::new(unsafe { Argv::new(argc, argv) }?.skip(1));
        if let Some(flag) = flags.next().transpose()? {
            match flag {
                Flag::Short('h') | Flag::Long("help") => {
                    eprintln!(
                        "empl [OPTIONS..]

Options:
  -h --help    Print this message and exit.
  -v --version Print version information and exit."
                    );
                    return Ok(());
                }
                Flag::Short('v') | Flag::Long("version") => {
                    eprintln!("{VERSION_MESSAGE}");
                    return Ok(());
                }
                flag => return Err(MainError::UnknownFlag(flag)),
            }
        }

        let playlists = SelectedConfig::get_playlists().ok_or(MainError::EmptyPlaylists)?;

        let runtime = runtime::Builder::new_current_thread()
            .build()
            .map_err(MainError::Runtime)?;
        runtime.block_on(async move {
            TaskManager::new(&playlists)
                .await
                .map_err(MainError::NewTaskManager)?
                .run()
                .await
                .map_err(MainError::Unrecoverable)
        })
    })() {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

#[derive(Debug)]
pub enum MainError {
    Arguments(ArgumentsError<'static, ArgError>),
    Argv(ArgvError),
    EmptyPlaylists,
    NewTaskManager(NewTaskManagerError),
    Runtime(io::Error),
    UnknownFlag(Flag<'static>),
    Unrecoverable(UnrecoverableError),
}
impl Display for MainError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Arguments(e) => e.fmt(f),
            Self::Argv(e) => e.fmt(f),
            Self::EmptyPlaylists => f.write_str("no playlists were found"),
            Self::NewTaskManager(e) => e.fmt(f),
            Self::Runtime(e) => write!(f, "failed to create async runtime: {e}"),
            Self::UnknownFlag(flag) => write!(f, "unknown flag `{flag}`"),
            Self::Unrecoverable(e) => e.fmt(f),
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
