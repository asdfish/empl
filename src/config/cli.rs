pub mod argv;
pub mod flag;

use {
    crate::{
        config::{
            ConfigStage, IntermediateConfig, KeyAction, UnknownKeyActionError,
            cli::{
                argv::{ArgError, Argv},
                flag::{Arguments, ArgumentsError, Flag},
            },
        },
        ext::pair::PairExt,
    },
    crossterm::style::{Color, Colors},
    nonempty_collections::{
        iter::{FromNonEmptyIterator, IntoIteratorExt, NonEmptyIterator},
        vector::NEVec,
    },
    std::{convert::Infallible, path::Path, sync::Arc},
};

pub struct CliConfig;
impl ConfigStage for CliConfig {
    type Error = CliError;
    type Next = Infallible;
    type Resources = Argv;

    fn execute(
        argv: Self::Resources,
    ) -> Option<
        Result<
            (
                IntermediateConfig,
                Option<<Self::Next as ConfigStage>::Resources>,
            ),
            Self::Error,
            >,
        > {
            fn check_pair<L, R>(l: &mut Option<L>, r: &mut Option<R>, output: &mut Vec<(L, R)>) {
                let l_is_some = l.is_some();
                let r_is_some = r.is_some();
                if let Some((l, r)) =
                (l.take_if(move |_| r_is_some), r.take_if(move |_| l_is_some)).transpose_option()
            {
                output.push((l, r));
            }
            }
            fn set<T>(config: &mut Option<T>) -> impl FnOnce(T) {
                move |item| {
                    *config = Some(item);
                }
            }
            fn set_colors(items: &mut State, config: &mut Colors) {
                [
                    (items.foreground.take(), &mut config.foreground),
                    (items.background.take(), &mut config.background),
                ]
                    .into_iter()
                    .for_each(|(mut color, into)| color.take().map(set(into)).unwrap_or_default());
            }
            fn value(
                args: &mut Arguments<'static, Argv, ArgError>,
                flag: Flag<'static>,
            ) -> Result<&'static str, CliError> {
                args.value()
                    .map(|res| {
                        res.map_err(ArgumentsError::Source)
                            .map_err(CliError::Arguments)
                    })
                    .unwrap_or(Err(CliError::MissingArgument(flag)))
            }

            let mut config = IntermediateConfig::default();
            let mut state = State::default();

            let mut arguments = Arguments::new(argv);
            while let Some(flag) = match arguments.next().transpose() {
                Ok(flag) => flag,
                Err(err) => return Some(Err(CliError::Arguments(err))),
            } {
                match match flag {
                    Flag::Short('b') | Flag::Long("background") => value(&mut arguments, flag)
                        .and_then(|color| {
                            Color::try_from(color).map_err(move |_| CliError::UnknownColor(color))
                        })
                        .map(set(&mut state.background)),
                    Flag::Short('f') | Flag::Long("foreground") => value(&mut arguments, flag)
                        .and_then(|color| {
                            Color::try_from(color).map_err(move |_| CliError::UnknownColor(color))
                        })
                        .map(set(&mut state.foreground)),

                    Flag::Short('P') | Flag::Long("playlist") => value(&mut arguments, flag)
                        .map(String::from)
                        .and_then(|playlist| {
                            state
                                .songs
                                .drain(..)
                                .try_into_nonempty_iter()
                                .ok_or(CliError::EmptyPlaylist)
                                .map(|iter| {
                                    iter.map(|song| {
                                        song.map_fst(String::from).map_snd(Arc::from)
                                    })
                                })
                                .map(NEVec::from_nonempty_iter)
                                .map(move |songs| (playlist, songs))
                        })
                        .map(|playlist| config.playlists.push(playlist)),
                Flag::Short('s') | Flag::Long("song-path") => value(&mut arguments, flag)
                    .map(Path::new)
                    .map(set(&mut state.song_path))
                    .map(|_| {
                        check_pair(&mut state.song_name, &mut state.song_path, &mut state.songs)
                    }),
                Flag::Short('n') | Flag::Long("song-name") => value(&mut arguments, flag)
                    .map(set(&mut state.song_name))
                    .map(|_| {
                        check_pair(&mut state.song_name, &mut state.song_path, &mut state.songs)
                    }),

                Flag::Short('a') | Flag::Long("action") => value(&mut arguments, flag)
                    .and_then(|key_action| {
                        KeyAction::parse(key_action).map_err(CliError::UnknownKeyAction)
                    })
                    .map(set(&mut state.key_action)),

                // Flag::Short('p') | Flag::Long("push") => {
                //     value(&mut arguments, flag).and_then(|field| match field {
                //         "cursor-colors" | "cc" => {
                //             set_colors(&mut state, &mut config.cursor_colors);
                //             Ok(())
                //         }
                //         "menu-colors" | "mc" => {
                //             set_colors(&mut state, &mut config.cursor_colors);
                //             Ok(())
                //         }
                //         "selection-colors" | "sc" => {
                //             set_colors(&mut state, &mut config.cursor_colors);
                //             Ok(())
                //         }

                //         field => Err(CliError::UnknownField(field)),
                //     })
                // }
                flag => Err(CliError::UnknownFlag(flag)),
            } {
                Ok(()) => (),
                Err(err) => return Some(Err(err)),
            }
        }

        Some(Ok((config, None)))
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CliError {
    Arguments(ArgumentsError<'static, ArgError>),
    EmptyPlaylist,
    MissingArgument(Flag<'static>),
    UnknownColor(&'static str),
    UnknownFlag(Flag<'static>),
    UnknownKeyAction(UnknownKeyActionError<&'static str>),
    UnknownField(&'static str),
    UnsetSongName,
    UnsetSongPath,
    UnsetPlaylist,
}

#[derive(Clone, Debug, Default)]
struct State {
    foreground: Option<Color>,
    background: Option<Color>,

    song_name: Option<&'static str>,
    song_path: Option<&'static Path>,
    songs: Vec<(&'static str, &'static Path)>,

    key_action: Option<KeyAction>,
}
