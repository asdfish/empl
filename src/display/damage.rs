use {
    crate::display::state::{DisplayState, Focus, Marker, Song},
    bumpalo::Bump,
    enum_iterator::Sequence,
    std::{
        io,
        marker::Unpin,
        ptr,
    },
    tokio::io::AsyncWriteExt,
};

#[derive(Clone, Copy, Debug, PartialEq, Sequence)]
pub enum Damage {
    Draw(Focus, Marker),
    Remove(Focus, Marker),
    FullRedraw,
    MoveOffset(Focus),
}
impl Damage {
    pub async fn execute<O>(&self, _: &Bump, _: &mut O, _old: &DisplayState<'_>, _new: &DisplayState<'_>) -> Result<(), io::Error>
    where O: AsyncWriteExt + Unpin {
        Ok(())
    }

    pub fn predicate(&self, old: &DisplayState, new: &DisplayState) -> bool {
        match self {
            Self::Draw(focus, marker) => {
                (marker.get(*focus, old) != marker.get(*focus, new))
                    && marker.get(*focus, new)
                        .map(|index| new.visible(*focus, index))
                        .unwrap_or_default()
            }
            Self::Remove(focus, marker) => {
                (marker.get(*focus, old) != marker.get(*focus, new))
                    && marker.get(*focus, old)
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
