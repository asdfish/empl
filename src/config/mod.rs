pub mod cli;
pub mod lisp;

use {
    crate::{
        config::{
            cli::{CliError, argv::Argv},
            lisp::{
                LispError,
                evaluator::{Arity, TryFromValue, Value},
                lexer::IntParser,
                parser::{Parser, ParserOutput, token::Just},
            },
        },
        ext::iterator::IteratorExt,
    },
    crossterm::{
        event::{KeyCode, KeyModifiers, MediaKeyCode, ModifierKeyCode},
        style::Colors,
    },
    nonempty_collections::vector::NEVec,
    std::{
        error::Error,
        fmt::{self, Display, Formatter},
        path::Path,
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

/// The version of the binary.
///
/// # Examples
///
/// ```
/// # use empl::config::VERSION;
/// if let Some(version) = option_env!("CARGO_PKG_VERSION") {
///      assert_eq!(version, VERSION);
/// }
/// ```
pub const VERSION: &str = "2.1.6";

fn parse_key_code<S>(key_code: S) -> Result<KeyCode, S>
where
    S: AsRef<str>,
{
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
        "media-play-pause" => Ok(KeyCode::Media(MediaKeyCode::PlayPause)),
        "media-reverse" => Ok(KeyCode::Media(MediaKeyCode::Reverse)),
        "media-stop" => Ok(KeyCode::Media(MediaKeyCode::Stop)),
        "media-fast-forward" => Ok(KeyCode::Media(MediaKeyCode::FastForward)),
        "media-rewind" => Ok(KeyCode::Media(MediaKeyCode::Rewind)),
        "media-track-next" => Ok(KeyCode::Media(MediaKeyCode::TrackNext)),
        "media-track-previous" => Ok(KeyCode::Media(MediaKeyCode::TrackPrevious)),
        "media-record" => Ok(KeyCode::Media(MediaKeyCode::Record)),
        "media-lower-volume" => Ok(KeyCode::Media(MediaKeyCode::LowerVolume)),
        "media-raise-volume" => Ok(KeyCode::Media(MediaKeyCode::RaiseVolume)),
        "media-mute-volume" => Ok(KeyCode::Media(MediaKeyCode::MuteVolume)),
        "left-shift" => Ok(KeyCode::Modifier(ModifierKeyCode::LeftShift)),
        "left-control" => Ok(KeyCode::Modifier(ModifierKeyCode::LeftControl)),
        "left-alt" => Ok(KeyCode::Modifier(ModifierKeyCode::LeftAlt)),
        "left-super" => Ok(KeyCode::Modifier(ModifierKeyCode::LeftSuper)),
        "left-hyper" => Ok(KeyCode::Modifier(ModifierKeyCode::LeftHyper)),
        "left-meta" => Ok(KeyCode::Modifier(ModifierKeyCode::LeftMeta)),
        "right-shift" => Ok(KeyCode::Modifier(ModifierKeyCode::RightShift)),
        "right-control" => Ok(KeyCode::Modifier(ModifierKeyCode::RightControl)),
        "right-alt" => Ok(KeyCode::Modifier(ModifierKeyCode::RightAlt)),
        "right-super" => Ok(KeyCode::Modifier(ModifierKeyCode::RightSuper)),
        "right-hyper" => Ok(KeyCode::Modifier(ModifierKeyCode::RightHyper)),
        "right-meta" => Ok(KeyCode::Modifier(ModifierKeyCode::RightMeta)),
        "iso-level-3-shift" => Ok(KeyCode::Modifier(ModifierKeyCode::IsoLevel3Shift)),
        "iso-level-5-shift" => Ok(KeyCode::Modifier(ModifierKeyCode::IsoLevel5Shift)),
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
fn parse_key_modifiers(key_modifier: &str) -> Result<KeyModifiers, char> {
    key_modifier
        .chars()
        .try_fold(KeyModifiers::NONE, |modifiers, ch| {
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
        })
}

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
    Lisp,
}
impl ConfigStage {
    pub const VARIANTS: [Self; 2] = [Self::Cli, Self::Lisp];

    pub fn execute(
        &self,
        resources: &mut Resources,
    ) -> Result<Option<IntermediateConfig>, ConfigError> {
        match self {
            Self::Cli => cli::execute(resources).map_err(ConfigError::Cli),
            Self::Lisp => lisp::execute(resources)
                .map(Some)
                .map_err(ConfigError::Lisp),
        }
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Cli(CliError),
    Lisp(LispError),
}
impl Display for ConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Cli(e) => e.fmt(f),
            Self::Lisp(e) => e.fmt(f),
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
