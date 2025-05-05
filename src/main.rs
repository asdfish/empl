#![cfg_attr(not(test), no_main)]

use {
    empl::{
        argv::{Argv, ArgvError},
        config::{Config, ConfigError},
    },
    std::{
        error::Error,
        ffi::{c_char, c_int},
        fmt::{self, Display, Formatter},
    },
};

#[cfg_attr(not(test), unsafe(no_mangle))]
extern "system" fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    match (move || -> Result<(), MainError> {
        let _config = Config::new(unsafe { Argv::new(argc, argv) }?)?;

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
    Argv(ArgvError),
    Config(ConfigError),
}
impl Display for MainError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Argv(e) => e.fmt(f),
            Self::Config(e) => e.fmt(f),
        }
    }
}
impl Error for MainError {}
impl From<ArgvError> for MainError {
    fn from(err: ArgvError) -> Self {
        Self::Argv(err)
    }
}
impl From<ConfigError> for MainError {
    fn from(err: ConfigError) -> Self {
        Self::Config(err)
    }
}
