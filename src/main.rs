#![cfg_attr(not(test), no_main)]

use {
    empl::{
        argv::{ArgError, ArgvError},
        config::{
            Config, EmptyConfigError, IntermediateConfig,
            clisp::{
                ast::{Expr, ExprParser},
                evaluator::Value,
                lexer::LexemeParser,
                parser::{Parser, ParserOutput},
            },
        },
        const_vec::CVec,
        ext::pair::PairExt,
        flag::{ArgumentsError, Flag},
        tasks::{NewTaskManagerError, TaskManager, UnrecoverableError},
    },
    nonempty_collections::iter::{IntoIteratorExt, NonEmptyIterator},
    std::{
        env,
        error::Error,
        ffi::{c_char, c_int},
        fmt::{self, Display, Formatter},
        fs, io,
        path::PathBuf,
        str,
    },
    tokio::runtime,
};

/// Not implemented as `concat!("empl ", env!("CARGO_PKG_VERSION"))` to allow compiling without cargo.
const VERSION_MESSAGE: &str = "empl 1.0.0";

const CONFIG_PATHS: [(&str, Option<&str>); 2] =
    [("XDG_CONFIG_HOME", None), ("HOME", Some(".config"))];

#[cfg_attr(not(test), unsafe(no_mangle))]
extern "system" fn main(_argc: c_int, _argv: *const *const c_char) -> c_int {
    match (move || -> Result<(), MainError> {
        //         let mut flags = Arguments::new(unsafe { Argv::new(argc, argv) }?.skip(1));
        //         if let Some(flag) = flags.next().transpose()? {
        //             match flag {
        //                 Flag::Short('h') | Flag::Long("help") => {
        //                     eprintln!(
        //                         "empl [OPTIONS..]

        // Options:
        //   -h --help    Print this message and exit.
        //   -v --version Print version information and exit."
        //                     );
        //                     return Ok(());
        //                 }
        //                 Flag::Short('v') | Flag::Long("version") => {
        //                     eprintln!("{VERSION_MESSAGE}");
        //                     return Ok(());
        //                 }
        //                 flag => return Err(MainError::UnknownFlag(flag)),
        //             }
        //         }

        let path = CONFIG_PATHS
            .into_iter()
            .find_map(|(var, dir)| {
                env::var_os(var).map(PathBuf::from).map(move |mut path| {
                    const CONFIG_PATH_TAIL: &[&str] = &["empl", "main.lisp"];
                    const CONFIG_PATH_TAIL_LENGTH: usize = const {
                        const fn string_len_sum(sum: usize, cons: &[&str]) -> usize {
                            match cons {
                                [car, cdr @ ..] => string_len_sum(car.len() + sum, cdr),
                                [] => sum,
                            }
                        }

                        string_len_sum(0, CONFIG_PATH_TAIL) +
                            // add path separators
                            CONFIG_PATH_TAIL.len()
                    };

                    path.reserve(dir.unwrap_or_default().len() + 1 + CONFIG_PATH_TAIL_LENGTH);
                    dir.into_iter()
                        .chain(CONFIG_PATH_TAIL.iter().copied())
                        .for_each(|tail| path.push(tail));

                    path
                })
            })
            .ok_or(MainError::ConfigPath)?;

        let config =
            fs::read_to_string(&path).map_err(move |err| MainError::ReadConfig(err, path))?;

        // TODO: error propagation
        let lexemes = LexemeParser.iter(&config).collect::<Vec<_>>();
        let expr = ExprParser
            .parse(&lexemes)
            .map(|ParserOutput { output, .. }| output)
            .unwrap_or(Expr::Value(Value::Unit));
        let config = IntermediateConfig::eval(expr)
            .map_err(|err| err.to_string())
            .map_err(MainError::EvalConfig)
            .and_then(|config| Config::try_from(config).map_err(MainError::IncompleteConfig))?;

        let runtime = runtime::Builder::new_current_thread()
            .build()
            .map_err(MainError::Runtime)?;
        runtime.block_on(async move {
            TaskManager::new(&config)
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
    ConfigPath,
    EmptyPlaylists,
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
            Self::ConfigPath => {
                let message =
                    const {
                        const HEAD: &str = if CONFIG_PATHS.len() == 1 {
                            "the environment variable [\""
                        } else {
                            "the environment variables [\""
                        };
                        const SEPARATOR: &str = "\", \"";
                        const TAIL: &str = if CONFIG_PATHS.len() == 1 {
                            "\"] is not set"
                        } else {
                            "\"] are not set"
                        };

                        const fn config_paths_len(
                            len: usize,
                            cons: &[(&str, Option<&str>)],
                        ) -> usize {
                            match cons {
                                [(car, _), cdr @ ..] => config_paths_len(len + car.len(), cdr),
                                [] => len,
                            }
                        }

                        let mut message = CVec::<
                            u8,
                            {
                                HEAD.len()
                                    + SEPARATOR.len() * (CONFIG_PATHS.len() - 1)
                                    + config_paths_len(0, &CONFIG_PATHS)
                                    + TAIL.len()
                            },
                        >::new();

                        message.concat(HEAD.as_bytes());
                        message.concat(CONFIG_PATHS[0].0.as_bytes());

                        let mut i = 1;
                        while i < CONFIG_PATHS.len() {
                            message.concat(SEPARATOR.as_bytes());
                            message.concat(CONFIG_PATHS[i].0.as_bytes());
                            i += 1;
                        }
                        message.concat(TAIL.as_bytes());

                        assert!(str::from_utf8(message.as_slice()).is_ok());
                        message
                    };

                // SAFETY: assertion above ensures safety
                f.write_str(unsafe { str::from_utf8_unchecked(message.as_slice()) })
            }
            Self::EmptyPlaylists => f.write_str("no playlists were found"),
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
