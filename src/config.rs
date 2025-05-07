//! Configuration file inspired by suckless programs.

use {
    dirs::home_dir,
    nonempty_collections::{
        iter::{IntoIteratorExt, NonEmptyIterator},
        NEVec,
    },
    std::{ffi::OsString, path::PathBuf},
};

pub type SelectedConfig = DefaultConfig;

#[expect(dead_code)]
const fn take_config<C: Config>() {}
const _: fn() = take_config::<SelectedConfig>;

pub trait Config {
    fn get_playlists() -> Option<NEVec<(OsString, NEVec<PathBuf>)>>;
}

pub struct DefaultConfig;
impl Config for DefaultConfig {
    fn get_playlists() -> Option<NEVec<(OsString, NEVec<PathBuf>)>> {
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
                    .map(|dir_ent| dir_ent.path())
                    .try_into_nonempty_iter()?
                    .collect::<NEVec<_>>();

                Some((dir_ent.file_name(), files))
            })
            .try_into_nonempty_iter()
            .map(NonEmptyIterator::collect::<NEVec<_>>)
    }
}
