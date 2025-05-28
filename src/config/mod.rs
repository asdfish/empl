pub mod cli;
pub mod clisp;

use {
    crate::{
        config::{
            cli::{CliError, argv::Argv},
            clisp::{
                CLispError,
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
        error::Error,
        fmt::{self, Display, Formatter},
        iter,
        path::Path,
        rc::Rc,
        sync::Arc,
    },
};

/// The name of the binary.
///
/// # Examples
///
/// ```
/// # use empl::config::NAME;
/// if let Some(name) = option_env!("CARGO_PKG_NAME") {
///      assert_eq!(name, NAME);
/// }
/// ```
pub const NAME: &str = "empl";

#[derive(Clone, Copy, Debug)]
pub struct Resources {
    argv: Argv,
    config_path: Option<&'static Path>,
}
impl Resources {
    pub const fn new(argv: Argv) -> Self {
        Self {
            argv,
            config_path: None,
        }
    }
}
impl From<Argv> for Resources {
    fn from(argv: Argv) -> Self {
        Self::new(argv)
    }
}
#[derive(Clone, Copy, Debug)]
pub enum ConfigStage {
    Cli,
    CLisp,
}
impl ConfigStage {
    pub const VARIANTS: [Self; 2] = [Self::Cli, Self::CLisp];

    pub fn execute(
        &self,
        resources: &mut Resources,
    ) -> Result<Option<IntermediateConfig>, ConfigError> {
        match self {
            Self::Cli => cli::execute(resources).map_err(ConfigError::Cli),
            Self::CLisp => clisp::execute(resources)
                .map(Some)
                .map_err(ConfigError::CLisp),
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Cli(CliError),
    CLisp(CLispError),
}
impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Cli(e) => e.fmt(f),
            Self::CLisp(e) => e.fmt(f),
        }
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
}
impl IntermediateConfig {
    fn new() -> Self {
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
impl Default for IntermediateConfig {
    fn default() -> Self {
        Self::new()
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
    T: AsRef<str>,
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
