pub mod cli;
pub mod clisp;

use {
    crate::{
        config::{
            cli::CliError,
            clisp::{
                ast::Expr,
                evaluator::{Arity, Environment, EvalError, List, TryFromValue, Value},
                lexer::IntParser,
                parser::{Parser, ParserOutput, token::Just},
            },
        },
        ext::{array::ArrayExt, iterator::IteratorExt},
        lazy_rc::LazyRc,
    },
    crossterm::{
        event::{KeyCode, KeyModifiers, MediaKeyCode, ModifierKeyCode},
        style::{Color, Colors},
    },
    nonempty_collections::{
        iter::{IntoIteratorExt, NonEmptyIterator},
        vector::NEVec,
    },
    qcell::{TCell, TCellOwner},
    std::{
        convert::Infallible,
        error::Error,
        fmt::{self, Display, Formatter},
        iter,
        path::Path,
        rc::Rc,
        sync::Arc,
    },
};

#[derive(Clone, Copy, Debug)]
pub enum ConfigError {
    Cli(CliError),
}
impl From<CliError> for ConfigError {
    fn from(err: CliError) -> Self {
        Self::Cli(err)
    }
}
impl From<Infallible> for ConfigError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}

pub trait ConfigStage {
    type Error: Into<ConfigError>;
    type Next: ConfigStage;
    type Resources;

    fn execute(
        _: Self::Resources,
    ) -> Option<
        Result<
            (
                IntermediateConfig,
                Option<<Self::Next as ConfigStage>::Resources>,
            ),
            Self::Error,
        >,
    >;
}
impl ConfigStage for Infallible {
    type Error = Infallible;
    type Next = Infallible;
    type Resources = Infallible;

    fn execute(
        _: Self::Resources,
    ) -> Option<
        Result<
            (
                IntermediateConfig,
                Option<<Self::Next as ConfigStage>::Resources>,
            ),
            Self::Error,
        >,
    > {
        unreachable!()
    }
}

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
    pub fn join(&mut self, new: Self) {
        macro_rules! join_color {
            ($colors:ident, $color:ident) => {
                if let Some($color) = new.$colors.$color {
                    self.$colors.$color = Some($color);
                }
            };
        }
        macro_rules! join_colors {
            ($colors:ident) => {
                join_color!($colors, foreground);
                join_color!($colors, background);
            };
            ($($colors:ident),* $(,)?) => {
                $(join_colors!($colors);)*
            };
        }
        join_colors![cursor_colors, menu_colors, selection_colors];
        self.key_bindings.extend(new.key_bindings);
        self.playlists.extend(new.playlists);
    }

    pub fn eval<'src>(expr: Expr<'src>) -> Result<Self, EvalError> {
        struct Id;

        let output = Rc::new(TCell::<Id, Self>::new(Self::default()));

        {
            let this = Rc::clone(&output);
            let mut environment = Environment::with_symbols(iter::once((
                "set-cfg!",
                Value::Fn(LazyRc::Owned(Rc::new(move |env, args| {
                    fn set_colors<'src>(
                        colors: &mut Colors,
                        value: Value<'src>,
                    ) -> Result<(), EvalError> {
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
                            fn parse_key_code<'src>(
                                key_code: LazyRc<'src, str>,
                            ) -> Result<KeyCode, LazyRc<'src, str>> {
                                // todo:
                                // F(u8),
                                // Char(char),
                                match key_code.as_ref() {
                                    "backspace" => Ok(KeyCode::Backspace),
                                    "enter" => Ok(KeyCode::Enter),
                                    "left" => Ok(KeyCode::Left),
                                    "right" => Ok(KeyCode::Right),
                                    "up" => Ok(KeyCode::Up),
                                    "down" => Ok(KeyCode::Down),
                                    "home" => Ok(KeyCode::Home),
                                    "end" => Ok(KeyCode::End),
                                    "page-up" => Ok(KeyCode::PageUp),
                                    "page-down" => Ok(KeyCode::PageDown),
                                    "tab" => Ok(KeyCode::Tab),
                                    "back-tab" => Ok(KeyCode::BackTab),
                                    "delete" => Ok(KeyCode::Delete),
                                    "insert" => Ok(KeyCode::Insert),
                                    "null" => Ok(KeyCode::Null),
                                    "esc" => Ok(KeyCode::Esc),
                                    "caps-lock" => Ok(KeyCode::CapsLock),
                                    "scroll-lock" => Ok(KeyCode::ScrollLock),
                                    "num-lock" => Ok(KeyCode::NumLock),
                                    "print-screen" => Ok(KeyCode::PrintScreen),
                                    "pause" => Ok(KeyCode::Pause),
                                    "menu" => Ok(KeyCode::Menu),
                                    "keypad-begin" => Ok(KeyCode::KeypadBegin),
                                    "media-play" => Ok(KeyCode::Media(MediaKeyCode::Play)),
                                    "media-pause" => Ok(KeyCode::Media(MediaKeyCode::Pause)),
                                    "media-play-pause" => {
                                        Ok(KeyCode::Media(MediaKeyCode::PlayPause))
                                    }
                                    "media-reverse" => Ok(KeyCode::Media(MediaKeyCode::Reverse)),
                                    "media-stop" => Ok(KeyCode::Media(MediaKeyCode::Stop)),
                                    "media-fast-forward" => {
                                        Ok(KeyCode::Media(MediaKeyCode::FastForward))
                                    }
                                    "media-rewind" => Ok(KeyCode::Media(MediaKeyCode::Rewind)),
                                    "media-track-next" => {
                                        Ok(KeyCode::Media(MediaKeyCode::TrackNext))
                                    }
                                    "media-track-previous" => {
                                        Ok(KeyCode::Media(MediaKeyCode::TrackPrevious))
                                    }
                                    "media-record" => Ok(KeyCode::Media(MediaKeyCode::Record)),
                                    "media-lower-volume" => {
                                        Ok(KeyCode::Media(MediaKeyCode::LowerVolume))
                                    }
                                    "media-raise-volume" => {
                                        Ok(KeyCode::Media(MediaKeyCode::RaiseVolume))
                                    }
                                    "media-mute-volume" => {
                                        Ok(KeyCode::Media(MediaKeyCode::MuteVolume))
                                    }
                                    "left-shift" => {
                                        Ok(KeyCode::Modifier(ModifierKeyCode::LeftShift))
                                    }
                                    "left-control" => {
                                        Ok(KeyCode::Modifier(ModifierKeyCode::LeftControl))
                                    }
                                    "left-alt" => Ok(KeyCode::Modifier(ModifierKeyCode::LeftAlt)),
                                    "left-super" => {
                                        Ok(KeyCode::Modifier(ModifierKeyCode::LeftSuper))
                                    }
                                    "left-hyper" => {
                                        Ok(KeyCode::Modifier(ModifierKeyCode::LeftHyper))
                                    }
                                    "left-meta" => Ok(KeyCode::Modifier(ModifierKeyCode::LeftMeta)),
                                    "right-shift" => {
                                        Ok(KeyCode::Modifier(ModifierKeyCode::RightShift))
                                    }
                                    "right-control" => {
                                        Ok(KeyCode::Modifier(ModifierKeyCode::RightControl))
                                    }
                                    "right-alt" => Ok(KeyCode::Modifier(ModifierKeyCode::RightAlt)),
                                    "right-super" => {
                                        Ok(KeyCode::Modifier(ModifierKeyCode::RightSuper))
                                    }
                                    "right-hyper" => {
                                        Ok(KeyCode::Modifier(ModifierKeyCode::RightHyper))
                                    }
                                    "right-meta" => {
                                        Ok(KeyCode::Modifier(ModifierKeyCode::RightMeta))
                                    }
                                    "iso-level-3-shift" => {
                                        Ok(KeyCode::Modifier(ModifierKeyCode::IsoLevel3Shift))
                                    }
                                    "iso-level-5-shift" => {
                                        Ok(KeyCode::Modifier(ModifierKeyCode::IsoLevel5Shift))
                                    }
                                    other => Just('f')
                                        .ignore_then(IntParser::<10, u8>::new())
                                        .parse(other)
                                        .map(ParserOutput::into_inner)
                                        .map(KeyCode::F)
                                        .or_else(|| {
                                            other
                                                .chars()
                                                .collect_array::<1>()
                                                .map(|[key]| KeyCode::Char(key))
                                        })
                                        .ok_or(key_code),
                                }
                            }
                            fn parse_key_modifier(
                                key_modifier: &str,
                            ) -> Result<KeyModifiers, char> {
                                key_modifier.chars().try_fold(
                                    KeyModifiers::NONE,
                                    |modifiers, ch| {
                                        match ch.to_ascii_lowercase() {
                                            'a' => Ok(KeyModifiers::ALT),
                                            'c' => Ok(KeyModifiers::CONTROL),
                                            'l' => Ok(KeyModifiers::SUPER),
                                            'h' => Ok(KeyModifiers::HYPER),
                                            'm' => Ok(KeyModifiers::META),
                                            's' => Ok(KeyModifiers::SHIFT),
                                            ch => Err(ch),
                                        }
                                        .map(move |modifier| modifiers.union(modifier))
                                    },
                                )
                            }

                            // '(string '(modifier key))
                            this.rw(&mut owner).key_bindings = Rc::<List<'src>>::try_from_value(value).map_err(EvalError::WrongType).and_then(|key_bindings| {
                                key_bindings
                                    .iter()
                                    .try_into_nonempty_iter()
                                    .ok_or(EvalError::WrongArity(Arity::RangeFrom(1..)))
                                    .and_then(|key_bindings| {
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
                                                        .and_then(|action| KeyAction::parse(action)
                                                            .map_err(|err| err.map(LazyRc::into_owned))
                                                            .map_err(EvalError::UnknownKeyAction))
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
                                                                                .and_then(|[modifier, key_code]| parse_key_modifier(modifier.as_ref())
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
impl KeyAction {
    fn parse<S>(key_action: S) -> Result<Self, UnknownKeyActionError<S>>
    where
        S: AsRef<str>,
    {
        match key_action.as_ref() {
            "quit" => Ok(Self::Quit),
            "move-up" => Ok(Self::MoveUp),
            "move-down" => Ok(Self::MoveDown),
            "move-left" => Ok(Self::MoveLeft),
            "move-right" => Ok(Self::MoveRight),
            "move-bottom" => Ok(Self::MoveBottom),
            "move-top" => Ok(Self::MoveTop),
            "move-selection" => Ok(Self::MoveSelection),
            "select" => Ok(Self::Select),
            "skip-song" => Ok(Self::SkipSong),
            _ => Err(UnknownKeyActionError(key_action)),
        }
    }
}
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct UnknownKeyActionError<S>(S)
where
    S: AsRef<str>;
impl<T> UnknownKeyActionError<T>
where
    T: AsRef<str>
{
    pub fn map<F, U>(self, f: F) -> UnknownKeyActionError<U>
    where
        F: FnOnce(T) -> U,
        U: AsRef<str>,
    {
        UnknownKeyActionError(f(self.0))
    }
}
impl<S> Display for UnknownKeyActionError<S>
where
    S: AsRef<str>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "unknown key action `{}`", self.0.as_ref())
    }
}
impl<S> Error for UnknownKeyActionError<S> where S: AsRef<str> + fmt::Debug {}
