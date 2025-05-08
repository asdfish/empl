//! Configuration file inspired by suckless programs.

use {
    crossterm::{
        event::{KeyCode, KeyModifiers},
        style::{Color, Colors},
    },
    dirs::home_dir,
    nonempty_collections::{
        iter::{IntoIteratorExt, NonEmptyIterator},
        NEVec,
    },
    std::{ffi::OsString, num::NonZeroUsize, path::PathBuf},
};

pub type SelectedConfig = DefaultConfig;

#[expect(dead_code)]
const fn take_config<C: Config>() {}
const _: fn() = take_config::<SelectedConfig>;

const fn get_max_key_binding_len(
    current_max: Option<usize>,
    cons: &'static [(KeyAction, &'static [(KeyModifiers, KeyCode)])],
) -> Option<usize> {
    match (cons, current_max) {
        ([(_, car), cdr @ ..], Some(current_max)) if car.len() > current_max => {
            get_max_key_binding_len(Some(car.len()), cdr)
        }
        ([_, cdr @ ..], Some(current_max)) => get_max_key_binding_len(Some(current_max), cdr),
        ([(_, car), cdr @ ..], None) => get_max_key_binding_len(Some(car.len()), cdr),
        ([], current_max) => current_max,
    }
}

pub type Playlists = NEVec<(String, NEVec<(String, PathBuf)>)>;

pub trait Config {
    const CURSOR_COLORS: Colors;
    const MENU_COLORS: Colors;
    const SELECTION_COLORS: Colors;

    const KEY_BINDINGS: &'static [(KeyAction, &'static [(KeyModifiers, KeyCode)])];
    const MAX_KEY_BINDING_LEN: NonZeroUsize =
        NonZeroUsize::new(get_max_key_binding_len(None, Self::KEY_BINDINGS).unwrap()).unwrap();

    fn get_playlists() -> Option<Playlists>;
}

pub struct DefaultConfig;
impl Config for DefaultConfig {
    const CURSOR_COLORS: Colors = Colors {
        foreground: Some(Color::Black),
        background: Some(Color::White),
    };
    const MENU_COLORS: Colors = Colors {
        foreground: Some(Color::White),
        background: Some(Color::Black),
    };
    const SELECTION_COLORS: Colors = Colors {
        foreground: Some(Color::Red),
        background: None,
    };

    const KEY_BINDINGS: &'static [(KeyAction, &'static [(KeyModifiers, KeyCode)])] = &[
        (
            KeyAction::Quit,
            &[(KeyModifiers::empty(), KeyCode::Char('q'))],
        ),
        (
            KeyAction::MoveUp(1),
            &[(KeyModifiers::empty(), KeyCode::Char('k'))],
        ),
        (
            KeyAction::MoveDown(1),
            &[(KeyModifiers::empty(), KeyCode::Char('j'))],
        ),
        (
            KeyAction::MoveLeft,
            &[(KeyModifiers::empty(), KeyCode::Char('h'))],
        ),
        (
            KeyAction::MoveRight,
            &[(KeyModifiers::empty(), KeyCode::Char('l'))],
        ),
        (
            KeyAction::Select,
            &[(KeyModifiers::empty(), KeyCode::Enter)],
        ),
    ];

    fn get_playlists() -> Option<Playlists> {
        fn os_string_to_string(os_string: OsString) -> String {
            os_string
                .into_string()
                .unwrap_or_else(|os_string| os_string.to_string_lossy().to_string())
        }

        home_dir()?
            .join("Music")
            .read_dir()
            .ok()?
            .flatten()
            .filter(|dir_ent| {
                dir_ent
                    .file_type()
                    .map(|file_type| file_type.is_dir())
                    .unwrap_or_default()
            })
            .flat_map(|dir_ent| {
                // do this first because this may short circuit before the file name, which can save an allocation
                let files = dir_ent
                    .path()
                    .read_dir()
                    .ok()?
                    .flatten()
                    .map(|dir_ent| {
                        (
                            dir_ent
                                .file_name()
                                .into_string()
                                .unwrap_or_else(os_string_to_string),
                            dir_ent.path(),
                        )
                    })
                    .try_into_nonempty_iter()?
                    .collect::<NEVec<_>>();

                Some((
                    dir_ent
                        .file_name()
                        .into_string()
                        .unwrap_or_else(os_string_to_string),
                    files,
                ))
            })
            .try_into_nonempty_iter()
            .map(NonEmptyIterator::collect::<NEVec<_>>)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum KeyAction {
    Quit,
    MoveUp(u16),
    MoveDown(u16),
    MoveLeft,
    MoveRight,
    Select,
}
