use {
    crate::display::state::{DisplayState, Focus, Marker},
    enum_iterator::Sequence,
};

#[derive(Clone, Copy, Debug, PartialEq, Sequence)]
pub enum Damage {
    Draw(Focus, Marker),
    Remove(Focus, Marker),
    FullRedraw,
    MoveOffset(Focus),
}
impl Damage {
    pub fn predicate(&self, old: &DisplayState, new: &DisplayState) -> bool {
        match self {
            Self::Draw(focus, marker) => {
                (marker.get(*focus, old) != marker.get(*focus, new))
                    && new.visible(*focus, marker.get(*focus, new))
            }
            Self::Remove(focus, marker) => {
                (marker.get(*focus, old) != marker.get(*focus, new))
                    && old.visible(*focus, marker.get(*focus, old))
            }
            Self::FullRedraw => {
                (old.terminal_area != new.terminal_area && new.terminal_area.is_some())
                    || old.selected_song.playlist != new.selected_song.playlist
            }
            Self::MoveOffset(focus) => {
                old.offsets[*focus] != new.offsets[*focus]
                    && (old.visible(*focus, old.offsets[*focus])
                        || new.visible(*focus, new.offsets[*focus]))
            }
        }
    }
}
