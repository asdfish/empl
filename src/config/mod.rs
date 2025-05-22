pub mod clisp;

use {
    crate::{
        config::clisp::{
            ast::{Expr, ExprParser},
            evaluator::{Arity, Environment, EvalError, List, TryFromValue, Value},
            lexer::LexemeParser,
            parser::{Parser, ParserOutput},
        },
        ext::{
            array::ArrayExt,
            iterator::IteratorExt,
        },
    },
    crossterm::{
        event::{KeyCode, KeyModifiers},
        style::{Color, Colors},
    },
    nonempty_collections::NEVec,
    qcell::{TCell, TCellOwner},
    std::{
        borrow::Cow,
        collections::HashMap,
        error::Error,
        fmt::{self, Display, Formatter},
        iter,
        path::Path,
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
            let mut environment = Environment::with_symbols(iter::once(
                ("set-cfg!", Value::Fn(Rc::new(move |env, args| {
                    fn set_colors<'src>(colors: &mut Colors, value: Value<'src>) -> Result<(), EvalError<'src>> {
                        let list = Rc::<List<'src>>::try_from_value(value)?;
                        let [foreground, background] = list.iter().collect_array().ok_or(EvalError::WrongListArity(Arity::Static(2)))?
                            .map(|color| Option::<Color>::try_from(color).map_err(EvalError::InvalidColor))
                            .transpose()?;

                        *colors = Colors {
                            foreground,
                            background,
                        };

                        Ok(())
                    }

                    let mut owner = TCellOwner::<Id>::new();
                    let [field, value] = args.into_iter().collect_array().ok_or(EvalError::WrongArity(Arity::Static(2)))?;
                    let field = env.eval_into::<Cow<Cow<str>>>(field)?;
                    let value = env.eval(value).map(Cow::into_owned)?;

                    match field.as_ref().as_ref() {
                        "cursor-colors" => {
                            set_colors(&mut this.rw(&mut owner).cursor_colors, value)?;
                        },
                        "menu-colors" => {
                            set_colors(&mut this.rw(&mut owner).menu_colors, value)?;
                        },
                        "selection-colors" => {
                            set_colors(&mut this.rw(&mut owner).selection_colors, value)?;
                        },
                        _ => return Err(EvalError::UnknownCfgField(field)),
                    }

                    Ok(Value::Unit)
                })))
            ));
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

// pub type SelectedConfig = DefaultConfig;

// #[expect(dead_code)]
// const fn take_config<C: Config>() {}
// const _: fn() = take_config::<SelectedConfig>;

// const fn get_max_key_binding_len(
//     current_max: Option<usize>,
//     cons: &'static [(KeyAction, &'static [(KeyModifiers, KeyCode)])],
// ) -> Option<usize> {
//     match (cons, current_max) {
//         ([(_, car), cdr @ ..], Some(current_max)) if car.len() > current_max => {
//             get_max_key_binding_len(Some(car.len()), cdr)
//         }
//         ([_, cdr @ ..], Some(current_max)) => get_max_key_binding_len(Some(current_max), cdr),
//         ([(_, car), cdr @ ..], None) => get_max_key_binding_len(Some(car.len()), cdr),
//         ([], current_max) => current_max,
//     }
// }

// pub trait Config {
//     const CURSOR_COLORS: Colors;
//     const MENU_COLORS: Colors;
//     const SELECTION_COLORS: Colors;

//     const KEY_BINDINGS: &'static [(KeyAction, &'static [(KeyModifiers, KeyCode)])];
//     const MAX_KEY_BINDING_LEN: NonZeroUsize =
//         NonZeroUsize::new(get_max_key_binding_len(None, Self::KEY_BINDINGS).unwrap()).unwrap();

//     fn get_playlists() -> Option<Playlists>;
// }

// pub struct DefaultConfig;
// impl Config for DefaultConfig {
//     const CURSOR_COLORS: Colors = Colors {
//         foreground: Some(Color::Black),
//         background: Some(Color::White),
//     };
//     const MENU_COLORS: Colors = Colors {
//         foreground: Some(Color::White),
//         background: Some(Color::Black),
//     };
//     const SELECTION_COLORS: Colors = Colors {
//         foreground: Some(Color::Red),
//         background: None,
//     };

//     const KEY_BINDINGS: &'static [(KeyAction, &'static [(KeyModifiers, KeyCode)])] = &[
//         (
//             KeyAction::Quit,
//             &[(KeyModifiers::empty(), KeyCode::Char('q'))],
//         ),
//         (
//             KeyAction::MoveUp,
//             &[(KeyModifiers::empty(), KeyCode::Char('k'))],
//         ),
//         (
//             KeyAction::MoveDown,
//             &[(KeyModifiers::empty(), KeyCode::Char('j'))],
//         ),
//         (
//             KeyAction::MoveLeft,
//             &[(KeyModifiers::empty(), KeyCode::Char('h'))],
//         ),
//         (
//             KeyAction::MoveRight,
//             &[(KeyModifiers::empty(), KeyCode::Char('l'))],
//         ),
//         (
//             KeyAction::MoveBottom,
//             &[(KeyModifiers::SHIFT, KeyCode::Char('G'))],
//         ),
//         (
//             KeyAction::MoveTop,
//             &[
//                 (KeyModifiers::empty(), KeyCode::Char('g')),
//                 (KeyModifiers::empty(), KeyCode::Char('g')),
//             ],
//         ),
//         (
//             KeyAction::MoveSelection,
//             &[(KeyModifiers::empty(), KeyCode::Char('r'))],
//         ),
//         (
//             KeyAction::Select,
//             &[(KeyModifiers::empty(), KeyCode::Enter)],
//         ),
//         (
//             KeyAction::SkipSong,
//             &[(KeyModifiers::empty(), KeyCode::Char('s'))],
//         ),
//     ];

//     fn get_playlists() -> Option<Playlists> {
//         fn os_string_to_string(os_string: OsString) -> String {
//             os_string
//                 .into_string()
//                 .unwrap_or_else(|os_string| os_string.to_string_lossy().to_string())
//         }

//         home_dir()?
//             .join("Music")
//             .read_dir()
//             .ok()?
//             .flatten()
//             .filter(|dir_ent| {
//                 dir_ent
//                     .file_type()
//                     .map(|file_type| file_type.is_dir())
//                     .unwrap_or_default()
//             })
//             .flat_map(|dir_ent| {
//                 // do this first because this may short circuit before the file name, which can save an allocation
//                 let files = dir_ent
//                     .path()
//                     .read_dir()
//                     .ok()?
//                     .flatten()
//                     .map(|dir_ent| {
//                         (
//                             dir_ent
//                                 .file_name()
//                                 .into_string()
//                                 .unwrap_or_else(os_string_to_string),
//                             Arc::from(dir_ent.path()),
//                         )
//                     })
//                     .try_into_nonempty_iter()?
//                     .collect::<NEVec<_>>();

//                 Some((
//                     dir_ent
//                         .file_name()
//                         .into_string()
//                         .unwrap_or_else(os_string_to_string),
//                     files,
//                 ))
//             })
//             .try_into_nonempty_iter()
//             .map(NonEmptyIterator::collect::<NEVec<_>>)
//     }
// }

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
