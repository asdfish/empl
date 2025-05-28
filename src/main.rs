#![cfg_attr(not(test), no_main)]

use {
    empl::{
        config::{
            Config, ConfigError, ConfigStage, EmptyConfigError, IntermediateConfig, Resources,
            cli::{
                argv::{ArgError, Argv, ArgvError},
                flag::{ArgumentsError, Flag},
            },
        },
        tasks::{NewTaskManagerError, TaskManager, UnrecoverableError},
    },
    std::{
        error::Error,
        ffi::{c_char, c_int},
        fmt::{self, Display, Formatter},
        io,
        path::PathBuf,
        process,
    },
    tokio::runtime,
};

#[cfg_attr(not(test), unsafe(no_mangle))]
extern "system" fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    match (move || -> Result<(), MainError> {
        unsafe { Argv::new(argc, argv) }
            .map_err(MainError::Argv)
            .map(Resources::new)
            .and_then(|resources| {
                ConfigStage::VARIANTS
                    .into_iter()
                    .try_fold(
                        (IntermediateConfig::default(), resources),
                        |(mut config, mut resources), stage| {
                            stage
                                .execute(&mut resources)
                                .map_err(Some)
                                .and_then(|config| config.ok_or(None))
                                .map(|new_config| config.join(new_config))
                                .map(move |_| (config, resources))
                        },
                    )
                    .map_or_else(
                        |err| match err.map(MainError::Config) {
                            Some(err) => Err(err),
                            None => Ok(None),
                        },
                        |(config, _)| Ok(Some(config)),
                    )
            })
            .map(|config| config.unwrap_or_else(|| process::exit(0)))
            .and_then(|config| Config::try_from(config).map_err(MainError::EmptyConfig))
            .and_then(|config| {
                runtime::Builder::new_current_thread()
                    .build()
                    .map_err(MainError::Runtime)
                    .map(move |runtime| (config, runtime))
            })
            .and_then(|(config, runtime)| {
                runtime.block_on(async move {
                    TaskManager::new(&config)
                        .await
                        .map_err(MainError::NewTaskManager)?
                        .run()
                        .await
                        .map_err(MainError::Unrecoverable)
                })
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
    Config(ConfigError),
    EmptyConfig(EmptyConfigError),
    EvalConfig(String),
    IncompleteConfig(EmptyConfigError),
    NewTaskManager(NewTaskManagerError),
    ReadConfig(io::Error, PathBuf),
    Runtime(io::Error),
    UnknownFlag(Flag<'static>),
    Unrecoverable(UnrecoverableError),
}
impl Display for MainError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Arguments(e) => e.fmt(f),
            Self::Argv(e) => e.fmt(f),
            Self::Config(err) => err.fmt(f),
            Self::EmptyConfig(e) => e.fmt(f),
            Self::EvalConfig(e) => write!(f, "failed to evaluate configuration file: {e}"),
            Self::IncompleteConfig(e) => e.fmt(f),
            Self::NewTaskManager(e) => e.fmt(f),
            Self::ReadConfig(e, path) => write!(
                f,
                "failed to read configuration file `{}`: {e}",
                path.display()
            ),
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
