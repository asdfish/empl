pub mod argv;
pub mod flag;

use {
    crate::{
        config::{
            IntermediateConfig, KeyAction, NAME, Resources, UnknownKeyActionError, VERSION,
            cli::{
                argv::ArgError,
                flag::{Arguments, ArgumentsError, Flag},
            },
            parse_key_code, parse_key_modifiers,
        },
        const_vec::ConstString,
        ext::pair::{
            BiTranspose,
            BiFunctor,
        },
    },
    crossterm::{
        event::{KeyCode, KeyModifiers},
        style::{Color, Colors},
    },
    nonempty_collections::{
        iter::{FromNonEmptyIterator, IntoIteratorExt, NonEmptyIterator},
        vector::NEVec,
    },
    std::{
        fmt::{self, Display, Formatter},
        mem,
        path::Path,
        sync::Arc,
    },
};

pub fn execute(resources: &mut Resources) -> Result<Option<IntermediateConfig>, CliError> {
    fn check_pair<L, R>(l: &mut Option<L>, r: &mut Option<R>, output: &mut Vec<(L, R)>) {
        let l_is_some = l.is_some();
        let r_is_some = r.is_some();
        if let Some((l, r)) =
            <(Option<L>, Option<R>) as BiTranspose<L, R>>::transpose((l.take_if(move |_| r_is_some), r.take_if(move |_| l_is_some)))
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
        args: &mut Arguments<
            'static,
            impl Iterator<Item = Result<&'static str, ArgError>>,
            ArgError,
        >,
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

    let mut arguments = Arguments::new(resources.argv.by_ref().skip(1));
    while let Some(flag) = arguments.next().transpose()? {
        match flag {
            Flag::Short('v') | Flag::Long("version") => {
                const MESSAGE: ConstString<{ NAME.len() + 1 + VERSION.len() }> = {
                    let mut message = ConstString::new();
                    message.push_str(NAME);
                    message.push(' ');
                    message.push_str(VERSION);

                    message
                };

                eprintln!("{}", MESSAGE);
                return Ok(None);
            }

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
            Flag::Short('c') | Flag::Long("color") => {
                value(&mut arguments, flag).and_then(|field| match field {
                    "cursor" | "c" => {
                        set_colors(&mut state, &mut config.cursor_colors);
                        Ok(())
                    }
                    "menu" | "m" => {
                        set_colors(&mut state, &mut config.cursor_colors);
                        Ok(())
                    }
                    "selection" | "s" => {
                        set_colors(&mut state, &mut config.cursor_colors);
                        Ok(())
                    }

                    field => Err(CliError::UnknownColorField(field)),
                })
            }

            Flag::Short('C') | Flag::Long("config") => value(&mut arguments, flag)
                .map(Path::new)
                .map(|path| state.config_path = Some(path)),

            Flag::Short('P') | Flag::Long("playlist") => value(&mut arguments, flag)
                .map(String::from)
                .and_then(|playlist| {
                    // not obtained using `mem::take` and `NEVec::try_from` because the iterator must be transformed
                    state
                        .songs
                        .drain(..)
                        .try_into_nonempty_iter()
                        .ok_or(CliError::EmptyPlaylist)
                        .map(|iter| iter.map(|song| song.map_fst(String::from).map_snd(Arc::from)))
                        .map(NEVec::from_nonempty_iter)
                        .map(move |songs| (playlist, songs))
                })
                .map(|playlist| config.playlists.push(playlist)),
            Flag::Short('s') | Flag::Long("song-path") => value(&mut arguments, flag)
                .map(Path::new)
                .map(set(&mut state.song_path))
                .map(|_| check_pair(&mut state.song_name, &mut state.song_path, &mut state.songs)),
            Flag::Short('n') | Flag::Long("song-name") => value(&mut arguments, flag)
                .map(set(&mut state.song_name))
                .map(|_| check_pair(&mut state.song_name, &mut state.song_path, &mut state.songs)),

            Flag::Short('a') | Flag::Long("key-action") => value(&mut arguments, flag)
                .and_then(|key_action| {
                    KeyAction::parse(key_action).map_err(CliError::UnknownKeyAction)
                })
                .and_then(|key_action| {
                    NEVec::try_from_vec(mem::take(&mut state.key_binding))
                        .ok_or(CliError::EmptyKeyBinding)
                        .map(move |key_binding| (key_action, key_binding))
                })
                .map(|key_binding| config.key_bindings.push(key_binding)),
            Flag::Short('k') | Flag::Long("key-code") => value(&mut arguments, flag)
                .and_then(|key_code| parse_key_code(key_code).map_err(CliError::UnknownKeyCode))
                .map(|key_code| state.key_code = Some(key_code))
                .map(|_| {
                    check_pair(
                        &mut state.key_modifiers,
                        &mut state.key_code,
                        &mut state.key_binding,
                    )
                }),
            Flag::Short('m') | Flag::Long("key-modifiers") => value(&mut arguments, flag)
                .and_then(|modifiers| {
                    parse_key_modifiers(modifiers).map_err(CliError::UnknownKeyModifier)
                })
                .map(|modifiers| state.key_modifiers = Some(modifiers))
                .map(|_| {
                    check_pair(
                        &mut state.key_modifiers,
                        &mut state.key_code,
                        &mut state.key_binding,
                    )
                }),

            flag => Err(CliError::UnknownFlag(flag)),
        }?
    }

    resources.config_path = state.config_path;

    Ok(Some(config))
}

#[derive(Clone, Copy, Debug)]
pub enum CliError {
    Arguments(ArgumentsError<'static, ArgError>),
    EmptyKeyBinding,
    EmptyPlaylist,
    MissingArgument(Flag<'static>),
    UnknownColor(&'static str),
    UnknownColorField(&'static str),
    UnknownFlag(Flag<'static>),
    UnknownKeyAction(UnknownKeyActionError<&'static str>),
    UnknownKeyCode(&'static str),
    UnknownKeyModifier(char),
    UnknownField(&'static str),
    UnsetSongName,
    UnsetSongPath,
    UnsetPlaylist,
}
impl Display for CliError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Arguments(e) => e.fmt(f),
            Self::EmptyKeyBinding => f.write_str("cannot have empty key bindings"),
            Self::EmptyPlaylist => f.write_str("cannot have empty playlist"),
            Self::MissingArgument(flag) => write!(f, "flag `{flag}` requires an argument"),
            Self::UnknownColor(color) => write!(f, "unknown color `{color}`"),
            Self::UnknownColorField(field) => write!(f, "unknown color field `{field}`"),
            Self::UnknownFlag(flag) => write!(f, "unknown flag `{flag}`"),
            Self::UnknownKeyAction(e) => e.fmt(f),
            Self::UnknownKeyCode(code) => write!(f, "unknown key code `{code}`"),
            Self::UnknownKeyModifier(modifier) => write!(f, "unknown key modifier `{modifier}`"),
            Self::UnknownField(field) => write!(f, "unknown field `{field}`"),
            Self::UnsetSongName => f.write_str("song name not set"),
            Self::UnsetSongPath => f.write_str("song path not set"),
            Self::UnsetPlaylist => f.write_str("playlist not set"),
        }
    }
}
impl From<ArgumentsError<'static, ArgError>> for CliError {
    fn from(err: ArgumentsError<'static, ArgError>) -> Self {
        Self::Arguments(err)
    }
}

#[derive(Clone, Debug, Default)]
struct State {
    foreground: Option<Color>,
    background: Option<Color>,

    song_name: Option<&'static str>,
    song_path: Option<&'static Path>,
    songs: Vec<(&'static str, &'static Path)>,

    key_code: Option<KeyCode>,
    key_modifiers: Option<KeyModifiers>,
    key_binding: Vec<(KeyModifiers, KeyCode)>,

    config_path: Option<&'static Path>,
}
