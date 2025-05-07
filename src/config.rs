//! Configuration file inspired by suckless programs.

use {
    crossterm::style::{Color, Colors},
    dirs::home_dir,
    nonempty_collections::{
        NEVec,
        iter::{IntoIteratorExt, NonEmptyIterator},
    },
    std::{ffi::OsString, path::PathBuf},
};

pub type SelectedConfig = DefaultConfig;

#[expect(dead_code)]
const fn take_config<C: Config>() {}
const _: fn() = take_config::<SelectedConfig>;

pub trait Config {
    const CURSOR_COLORS: Colors;
    const MENU_COLORS: Colors;
    const SELECTION_COLORS: Colors;

    fn get_playlists() -> Option<NEVec<(String, NEVec<(String, PathBuf)>)>>;
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

    fn get_playlists() -> Option<NEVec<(String, NEVec<(String, PathBuf)>)>> {
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
