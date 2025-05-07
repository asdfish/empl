use {
    crate::{
        display::state::{DisplayState, Focus, Marker, Song},
        ext::command::CommandChain,
    },
    either::Either,
    std::ptr,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Damage {
    Draw(Focus, Marker),
    Remove(Focus, Marker),
    MoveOffset(Focus),
    FullRedraw,
}
impl Damage {
    fn render<O>(
        &self,
        old: &DisplayState<'_>,
        new: &DisplayState<'_>,
    ) -> impl CommandChain {
        match self {
            Self::Draw(focus, marker) => Either::Left(
                marker
                    .get(*focus, new)
                    .map(|index| new.render_line(*focus, index)),
            ),
            Self::Remove(focus, marker) => Either::Right(Either::Left(
                marker
                    .get(*focus, old)
                    .map(|index| new.render_line(*focus, index)),
            )),
            Self::FullRedraw => Either::Right(Either::Right(Either::Left(
                new.render_menu(Focus::Playlists)
                    .then(new.render_menu(Focus::Songs)),
            ))),
            Self::MoveOffset(focus) => Either::Right(Either::Right(Either::Right(
                new.render_menu(*focus),
            ))),
        }
    }

    pub fn predicate(&self, old: &DisplayState, new: &DisplayState) -> bool {
        match self {
            Self::Draw(focus, marker) => {
                (marker.get(*focus, old) != marker.get(*focus, new))
                    && marker
                        .get(*focus, new)
                        .map(|index| new.visible(*focus, index))
                        .unwrap_or_default()
            }
            Self::Remove(focus, marker) => {
                (marker.get(*focus, old) != marker.get(*focus, new))
                    && marker
                        .get(*focus, old)
                        .map(|index| old.visible(*focus, index))
                        .unwrap_or_default()
            }
            Self::FullRedraw => {
                (old.terminal_area != new.terminal_area && new.terminal_area.is_some())
                    || matches!((old.selected_song, new.selected_song), (Some(Song { playlist: old_playlist, .. }), Some(Song { playlist: new_playlist, .. })) if old_playlist != new_playlist)
                    || ptr::from_ref(old.playlists) != ptr::from_ref(new.playlists)
            }
            Self::MoveOffset(focus) => {
                old.offsets[*focus] != new.offsets[*focus]
                    && (old.visible(*focus, old.offsets[*focus])
                        || new.visible(*focus, new.offsets[*focus]))
            }
        }
    }
}
