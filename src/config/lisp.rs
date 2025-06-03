//! custom lisp dialect used for configurating this program

pub mod ast;
pub mod evaluator;
pub mod lexer;
pub mod parser;

use {
    crate::{
        config::{
            Arity, IntermediateConfig, KeyAction, NAME, Resources, TryFromValue, Value,
            lisp::{
                ast::ExprParser,
                evaluator::{Environment, EvalError, List},
                lexer::LexemeParser,
                parser::{Parser, ParserOutput},
            },
            parse_key_code, parse_key_modifiers,
        },
        ext::{array::ArrayExt, iterator::IteratorExt},
        lazy_rc::LazyRc,
    },
    crossterm::style::{Color, Colors},
    nonempty_collections::{
        iter::{IntoIteratorExt, NonEmptyIterator},
        vector::NEVec,
    },
    qcell::{TCell, TCellOwner},
    std::{
        borrow::Cow,
        env,
        fmt::{self, Display, Formatter},
        fs, io, iter,
        path::{Path, PathBuf},
        rc::Rc,
        sync::Arc,
    },
};

const CONFIG_FILE_NAME: &str = "main.lisp";
const ENV_CONFIG_PATHS: [(&str, Option<&str>); 2] =
    [("XDG_CONFIG_HOME", None), ("HOME", Some(".config"))];

pub fn execute(resources: &mut Resources) -> Result<IntermediateConfig, LispError> {
    resources
        .config_path
        .map(Cow::Borrowed)
        .or_else(|| {
            ENV_CONFIG_PATHS
                .into_iter()
                .find_map(|(env, suffix)| {
                    env::var_os(env).map(PathBuf::from).map(|mut path| {
                        path.reserve(
                            suffix.map(|suffix| {
                                // `/`
                                1 + suffix.len()
                            })
                                .unwrap_or_default()
                                // `/`
                                + 1
                                + NAME.len()
                                // '/'
                                + 1
                                + CONFIG_FILE_NAME.len(),
                        );
                        suffix.into_iter()
                            .chain([NAME, CONFIG_FILE_NAME])
                            .for_each(|suffix| path.push(suffix));
                        path
                    })
                })
                .map(Cow::Owned)
        })
        .ok_or(LispError::UnsetEnvVars)
        .and_then(|path| fs::read_to_string(&path).map_err(move |err| LispError::ReadConfig(err, path)))
        .and_then(|config| {
            let lexemes = LexemeParser.iter(&config).collect::<Vec<_>>();
            ExprParser.parse(&lexemes).ok_or(LispError::InvalidSyntax)
                .map(ParserOutput::into_inner)
                .and_then(|expr| {


                    struct Id;

                    let output = Rc::new(TCell::<Id, IntermediateConfig>::new(
                        IntermediateConfig::default(),
                    ));

                    {
                        let this = Rc::clone(&output);
                        let mut environment = Environment::with_symbols(iter::once((
            "set-cfg!",
            Value::Fn(LazyRc::Owned(Rc::new(move |env, args| {
                fn set_colors(colors: &mut Colors, value: Value) -> Result<(), EvalError> {
                    let list = Rc::<List>::try_from_value(value)?;
                    let [foreground, background] = list
                        .iter()
                        .collect_array()
                        .ok_or(EvalError::WrongListArity(Arity::Static(2)))?
                        .map(|color| {
                            Option::<Color>::try_from(color).map_err(EvalError::InvalidColor)
                        })
                        .transpose()?;

                    *colors = Colors {
                        foreground,
                        background,
                    };

                    Ok(())
                }

                let mut owner = TCellOwner::<Id>::new();
                let [field, value] = args
                    .into_iter()
                    .collect_array()
                    .ok_or(EvalError::WrongArity(Arity::Static(2)))?;
                let field = env.eval_into::<LazyRc<str>>(field)?;
                let value = env.eval(value)?;

                match field.as_ref() {
                    "cursor-colors" => {
                        set_colors(&mut this.rw(&mut owner).cursor_colors, value)?;
                    }
                    "menu-colors" => {
                        set_colors(&mut this.rw(&mut owner).menu_colors, value)?;
                    }
                    "selection-colors" => {
                        set_colors(&mut this.rw(&mut owner).selection_colors, value)?;
                    }
                    "key-bindings" => {
                        // '(string '(modifier key))
                        this.rw(&mut owner).key_bindings = Rc::<List>::try_from_value(value).map_err(EvalError::WrongType).and_then(|key_bindings| {
                            key_bindings
                                .iter()
                                .try_into_nonempty_iter()
                                .ok_or(EvalError::WrongArity(Arity::RangeFrom(1..)))
                                .and_then(|key_bindings| {
                                    key_bindings.map(|key_binding| {
                                        Rc::<List>::try_from_value(key_binding)
                                            .map_err(EvalError::WrongType)
                                            .and_then(|key_binding| {
                                                key_binding.iter().collect_array::<2>().ok_or(
                                                    EvalError::WrongListArity(Arity::Static(2)),
                                                )
                                            })
                                            .and_then(|[action, keys]| {
                                                LazyRc::<str>::try_from_value(action)
                                                    .map_err(EvalError::WrongType)
                                                    .and_then(|action| KeyAction::parse(action)
                                                        .map_err(|err| err.map(LazyRc::into_owned))
                                                        .map_err(EvalError::UnknownKeyAction))
                                                    .and_then(move |action| {
                                                        Rc::<List>::try_from_value(keys)
                                                            .map_err(EvalError::WrongType)
                                                            .and_then(|keys| {
                                                                keys
                                                                    .iter()
                                                                    .try_into_nonempty_iter()
                                                                    .ok_or(EvalError::WrongListArity(Arity::RangeFrom(1..)))
                                                            })
                                                            .map(|keys| {
                                                                keys
                                                                    .map(|key| {
                                                                        Rc::<List>::try_from_value(key)
                                                                            .map_err(EvalError::WrongType)
                                                                            .and_then(|key| {
                                                                                key
                                                                                    .iter()
                                                                                    .collect_array::<2>()
                                                                                    .ok_or(EvalError::WrongListArity(Arity::Static(2)))
                                                                            })
                                                                            .and_then(|key| {
                                                                                key
                                                                                    .map(LazyRc::<str>::try_from_value)
                                                                                    .transpose()
                                                                                    .map_err(EvalError::WrongType)
                                                                            })
                                                                            .and_then(|[modifier, key_code]| parse_key_modifiers(modifier.as_ref())
                                                                                .map_err(EvalError::UnknownKeyModifier)
                                                                                .and_then(move |modifier| parse_key_code(key_code)
                                                                                    .map_err(LazyRc::into_owned)
                                                                                    .map_err(EvalError::UnknownKeyCode)
                                                                                    .map(move |key_code| (modifier, key_code))))
                                                                    })
                                                            })
                                                            .and_then(|keys| keys.collect::<Result<NEVec<_>, _>>()
                                                                .map(move |keys| (action, keys)))
                                                    })
                                            })
                                    })
                                        .collect::<Result<Vec<_>, _>>()
                                })
                        })?;
                    }
                    "playlists" => {
                                        // '(string (string path))
                        let value = Rc::<List>::try_from_value(value)?;
                        this.rw(&mut owner).playlists = value
                            .iter()
                            .map(|playlist| {
                                Rc::<List>::try_from_value(playlist)
                                    .map_err(EvalError::WrongType)
                                    .and_then(|playlist| {
                                        playlist
                                            .iter()
                                            .collect_array::<2>()
                                            .ok_or(EvalError::WrongListArity(Arity::Static(2)))
                                    })
                                    .and_then(|[name, songs]| {
                                        LazyRc::<str>::try_from_value(name)
                                            .map(|name| name.to_string())
                                            .map_err(EvalError::WrongType)
                                            .and_then(move |name| {
                                                Rc::<List>::try_from_value(songs)
                                                    .map_err(EvalError::WrongType)
                                                    .and_then(|songs| {
                                                        songs.iter()
                                                            .map(|song| Rc::<List>::try_from_value(song)
                                                                .map_err(EvalError::WrongType)
                                                                .and_then(|song|
                                                                        song
                                                                            .iter()
                                                                            .collect_array::<2>()
                                                                            .ok_or(EvalError::WrongListArity(Arity::Static(2))))
                                                                .and_then(|[name, path]| LazyRc::<str>::try_from_value(name)
                                                                    .map(|name| name.to_string())
                                                                    .and_then(move |name| LazyRc::<Path>::try_from_value(path)
                                                                        .map(|path| -> Arc<Path> {
                                                                            match path {
                                                                                LazyRc::Borrowed(path) => Arc::from(path),
                                                                                LazyRc::Owned(path) => Arc::from(path.as_ref()),
                                                                            }
                                                                        })
                                                                        .map(move |path| (name, path)))
                                                                    .map_err(EvalError::WrongType)))
                                                            .try_into_nonempty_iter().ok_or(EvalError::WrongListArity(Arity::RangeFrom(1..)))
                                                    })
                                                    .and_then(NonEmptyIterator::collect::<Result<NEVec<_>, _>>)
                                                    .map(move |songs| (name, songs))
                                            })
                                    })
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                    }
                    _ => return Err(EvalError::UnknownCfgField(field.into_owned())),
                }

                Ok(Value::Unit)
            }))),
                        )));
                        environment.eval(expr)?;
                    }

                    Ok(Rc::into_inner(output).unwrap().into_inner())
                })
        })
}

#[derive(Debug)]
pub enum LispError {
    Eval(EvalError),
    InvalidSyntax,
    ReadConfig(io::Error, Cow<'static, Path>),
    UnsetEnvVars,
}
impl Display for LispError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Eval(e) => e.fmt(f),
            Self::InvalidSyntax => f.write_str("invalid syntax"),
            Self::ReadConfig(err, path) => write!(
                f,
                "failed to read configuration at `{}`: {err}",
                path.display()
            ),
            Self::UnsetEnvVars => f.write_str("unset environment variables"),
        }
    }
}
impl From<EvalError> for LispError {
    fn from(err: EvalError) -> Self {
        Self::Eval(err)
    }
}
