pub mod clisp;

use {
    crate::{
        config::clisp::{
            ast::Expr,
            evaluator::{Arity, Environment, EvalError, List, TryFromValue, Value},
        },
        ext::{array::ArrayExt, iterator::IteratorExt},
        lazy_rc::LazyRc,
    },
    crossterm::{
        event::{KeyCode, KeyModifiers},
        style::{Color, Colors},
    },
    nonempty_collections::{
        iter::{IntoIteratorExt, NonEmptyIterator},
        vector::NEVec,
    },
    qcell::{TCell, TCellOwner},
    std::{
        error::Error,
        fmt::{self, Display, Formatter},
        iter,
        path::Path,
        rc::Rc,
        sync::Arc,
    },
};

pub type KeyBinding = (KeyAction, NEVec<(KeyModifiers, KeyCode)>);
pub type Playlist = (String, NEVec<(String, Arc<Path>)>);

#[derive(Debug)]
pub struct IntermediateConfig {
    cursor_colors: Colors,
    menu_colors: Colors,
    selection_colors: Colors,
    key_bindings: Vec<KeyBinding>,
    playlists: Vec<Playlist>,
}
impl IntermediateConfig {
    pub fn eval<'src>(expr: Expr<'src>) -> Result<Self, EvalError<'src>> {
        struct Id;

        let output = Rc::new(TCell::<Id, Self>::new(Self::default()));

        {
            let this = Rc::clone(&output);
            let mut environment = Environment::with_symbols(iter::once((
                "set-cfg!",
                Value::Fn(Rc::new(move |env, args| {
                    fn set_colors<'src>(
                        colors: &mut Colors,
                        value: Value<'src>,
                    ) -> Result<(), EvalError<'src>> {
                        let list = Rc::<List<'src>>::try_from_value(value)?;
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
                    let field = env.eval_into::<LazyRc<'src, str>>(field)?;
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
                            Rc::<List<'src>>::try_from_value(value).and_then(|key_bindings| {
                                key_bindings
                                    .iter()
                                    .try_into_nonempty_iter()
                                    .ok_or(EvalError::WrongArity(Arity::RangeFrom(1..)))
                                    .map(|key_bindings| {
                                        key_bindings.map(|key_binding| {
                                            Rc::<List<'src>>::try_from_value(key_binding)
                                                .map_err(EvalError::WrongType)
                                                .and_then(|key_binding| {
                                                    key_binding.iter().collect_array::<2>().ok_or(
                                                        EvalError::WrongListArity(Arity::Static(2)),
                                                    )
                                                })
                                                .and_then(|[action, keys]| {
                                                    LazyRc::<'src, str>::try_from_value(action)
                                                        .map_err(EvalError::WrongType)
                                                        .and_then(|action| KeyAction::try_from(action).map_err(EvalError::UnknownKeyAction))
                                                        .and_then(move |action| {
                                                            Rc::<List<'src>>::try_from_value(keys)
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
                                                                         Rc::<List<'src>>::try_from_value(key)
                                                                          .map_err(EvalError::WrongType)
                                                                          .and_then(|key| {
                                                                               key
                                                                                   .iter()
                                                                                   .collect_array::<2>()
                                                                                   .ok_or(EvalError::WrongListArity(Arity::Static(2)))
                                                                          })
                                                                         .and_then(|key| {
                                                                             key
                                                                                 .map(LazyRc::<'src, str>::try_from_value)
                                                                                 .transpose()
                                                                                 .map_err(EvalError::WrongType)
                                                                         })
                                                                     })
                                                                })
                                                                .map(move |keys| (action, keys))
                                                        })
                                                })
                                        })
                                    });

                                Ok(())
                            })?;
                        }
                        "playlists" => {
                            let value = Rc::<List<'src>>::try_from_value(value)?;
                            this.rw(&mut owner).playlists = value
                                .iter()
                                .map(|playlist| {
                                    Rc::<List<'src>>::try_from_value(playlist)
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
                                                    Rc::<List<'src>>::try_from_value(songs)
                                                        .map_err(EvalError::WrongType)
                                                        .and_then(|songs| {
                                                            songs.iter()
                                                                .map(|song| Rc::<List<'src>>::try_from_value(song)
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
                                .collect::<Result<Vec<_>, _>>()?
;
                        }
                        _ => return Err(EvalError::UnknownCfgField(field)),
                    }

                    Ok(Value::Unit)
                })),
            )));
            environment.eval(expr)?;
        }

        Ok(Rc::into_inner(output).unwrap().into_inner())
    }
}
impl Default for IntermediateConfig {
    fn default() -> Self {
        Self {
            cursor_colors: Colors {
                foreground: None,
                background: None,
            },
            menu_colors: Colors {
                foreground: None,
                background: None,
            },
            selection_colors: Colors {
                foreground: None,
                background: None,
            },
            key_bindings: Vec::new(),
            playlists: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    pub cursor_colors: Colors,
    pub menu_colors: Colors,
    pub selection_colors: Colors,
    pub key_bindings: NEVec<KeyBinding>,
    pub playlists: NEVec<Playlist>,
}
impl TryFrom<IntermediateConfig> for Config {
    type Error = EmptyConfigError;

    fn try_from(
        IntermediateConfig {
            cursor_colors,
            menu_colors,
            selection_colors,
            key_bindings,
            playlists,
            ..
        }: IntermediateConfig,
    ) -> Result<Self, EmptyConfigError> {
        NEVec::try_from_vec(key_bindings)
            .ok_or(EmptyConfigError::KeyBindings)
            .and_then(move |key_bindings| {
                NEVec::try_from_vec(playlists)
                    .ok_or(EmptyConfigError::Playlists)
                    .map(move |playlists| (key_bindings, playlists))
            })
            .map(|(key_bindings, playlists)| Self {
                cursor_colors,
                menu_colors,
                selection_colors,
                key_bindings,
                playlists,
            })
    }
}
#[derive(Clone, Copy, Debug)]
pub enum EmptyConfigError {
    KeyBindings,
    Playlists,
}
impl Display for EmptyConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "{} cannot be empty",
            match self {
                Self::KeyBindings => "key bindings",
                Self::Playlists => "playlists",
            }
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum KeyAction {
    Quit,
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    MoveBottom,
    MoveTop,
    MoveSelection,
    Select,
    SkipSong,
}
impl<'a> TryFrom<LazyRc<'a, str>> for KeyAction {
    type Error = UnknownKeyActionError<'a>;

    fn try_from(key_action: LazyRc<'a, str>) -> Result<Self, UnknownKeyActionError<'a>> {
        match key_action.as_ref() {
            "quit" => Ok(Self::Quit),
            "move_up" => Ok(Self::MoveUp),
            "move_down" => Ok(Self::MoveDown),
            "move_left" => Ok(Self::MoveLeft),
            "move_right" => Ok(Self::MoveRight),
            "move_bottom" => Ok(Self::MoveBottom),
            "move_top" => Ok(Self::MoveTop),
            "move_selection" => Ok(Self::MoveSelection),
            "select" => Ok(Self::Select),
            "skip_song" => Ok(Self::SkipSong),
            _ => Err(UnknownKeyActionError(key_action)),
        }
    }
}
#[derive(Clone, Debug)]
pub struct UnknownKeyActionError<'a>(LazyRc<'a, str>);
impl Display for UnknownKeyActionError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "unknown key action `{}`", self.0.as_ref())
    }
}
impl Error for UnknownKeyActionError<'_> {}
