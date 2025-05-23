pub mod clisp;

use {
    crate::{
        config::clisp::{
            ast::Expr,
            evaluator::{Arity, Environment, EvalError, List, TryFromValue, Value},
        },
        ext::{array::ArrayExt, iterator::IteratorExt},
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
        borrow::Cow,
        error::Error,
        fmt::{self, Display, Formatter},
        iter,
        path::{self, Path, PathBuf},
        rc::Rc,
        sync::Arc,
    },
};

#[derive(Debug)]
pub struct IntermediateConfig {
    cursor_colors: Colors,
    menu_colors: Colors,
    selection_colors: Colors,
    key_bindings: Vec<(KeyAction, NEVec<(KeyModifiers, KeyCode)>)>,
    playlists: Vec<(String, NEVec<(String, Arc<Path>)>)>,
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
                    let field = env.eval_into::<Cow<'src, Cow<'src, str>>>(field)?;
                    let value = env.eval(value)?;

                    match field.as_ref().as_ref() {
                        "cursor-colors" => {
                            set_colors(&mut this.rw(&mut owner).cursor_colors, value)?;
                        }
                        "menu-colors" => {
                            set_colors(&mut this.rw(&mut owner).menu_colors, value)?;
                        }
                        "selection-colors" => {
                            set_colors(&mut this.rw(&mut owner).selection_colors, value)?;
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
                                            Cow::<'src, Cow<'src, str>>::try_from_value(name)
                                                .map(Cow::into_owned)
                                                .map(Cow::into_owned)
                                                .map_err(EvalError::WrongType)
                                                .and_then(move |name| {
                                                    Rc::<List<'src>>::try_from_value(songs)
                                                        .map_err(EvalError::WrongType)
                                                        .and_then(|songs| {
                                                            songs.iter()
                                                                .map(|song| Cow::<'src, Cow<'src, str>>::try_from_value(song)
                                                                    .map(Cow::into_owned)
                                                                    .map(Cow::into_owned)
                                                                    .map(|path| (path.rsplit_once(path::MAIN_SEPARATOR)
                                                                        .map(|(_, tail)| tail)
                                                                        .unwrap_or(&path).to_string(), Arc::<Path>::from(PathBuf::from(path))))
                                                                    .map_err(EvalError::WrongType)
                                                                )
                                                                .try_into_nonempty_iter().ok_or(EvalError::WrongListArity(Arity::RangeFrom(1..)))
                                                        })
                                                        .and_then(NonEmptyIterator::collect::<Result<NEVec<_>, _>>)
                                                        .map(move |songs| (name, songs))
                                                })
                                        })
                                })
                                .collect::<Result<Vec<_>, _>>()?;
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
    pub key_bindings: NEVec<(KeyAction, NEVec<(KeyModifiers, KeyCode)>)>,
    pub playlists: Playlists,
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
pub type Playlists = NEVec<(String, NEVec<(String, Arc<Path>)>)>;

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
impl<'a> TryFrom<&'a str> for KeyAction {
    type Error = UnknownKeyActionError<'a>;

    fn try_from(key_action: &'a str) -> Result<Self, UnknownKeyActionError<'a>> {
        match key_action {
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
            key_action => Err(UnknownKeyActionError(key_action)),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct UnknownKeyActionError<'a>(&'a str);
impl Display for UnknownKeyActionError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "unknown key action `{}`", self.0)
    }
}
impl Error for UnknownKeyActionError<'_> {}
