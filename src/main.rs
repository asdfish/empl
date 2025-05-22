#![cfg_attr(not(test), no_main)]

use {
    empl::{
        argv::{ArgError, Argv, ArgvError},
        config::{
            Config, EmptyConfigError, IntermediateConfig,
            clisp::{
                ast::{Expr, ExprParser},
                evaluator::Value,
                lexer::LexemeParser,
                parser::{Parser, ParserOutput},
            },
        },
        flag::{Arguments, ArgumentsError, Flag},
        tasks::{NewTaskManagerError, TaskManager, UnrecoverableError},
    },
    std::{
        error::Error,
        ffi::{c_char, c_int},
        fmt::{self, Display, Formatter},
        fs, io,
        path::Path,
    },
    tokio::runtime,
};

/// Not implemented as `concat!("empl ", env!("CARGO_PKG_VERSION"))` to allow compiling without cargo.
const VERSION_MESSAGE: &str = "empl 1.0.0";

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

        let config =
            // TODO: change file path
            fs::read_to_string(Path::new("main.lisp"))
                .map_err(MainError::ReadConfig)?;

        // TODO: error propagation
        let lexemes = LexemeParser.iter(&config).collect::<Vec<_>>();
        let mut expr = ExprParser
            .parse(&lexemes)
            .map(|ParserOutput { output, .. }| output)
            .unwrap_or(Expr::Value(Value::Unit));
        let config = IntermediateConfig::eval(expr).map_err(|err| err.to_string()).map_err(MainError::EvalConfig)
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
    EmptyPlaylists,
    EvalConfig(String),
    IncompleteConfig(EmptyConfigError),
    NewTaskManager(NewTaskManagerError),
    ReadConfig(io::Error),
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
            Self::EvalConfig(e) => write!(f, "failed to evaluate configuration file: {e}"),
            Self::IncompleteConfig(e) => e.fmt(f),
            Self::NewTaskManager(e) => e.fmt(f),
            Self::ReadConfig(e) => write!(f, "failed to read configuration file: {e}"),
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
