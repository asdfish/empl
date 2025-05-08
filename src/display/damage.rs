use {
    crate::{
        display::state::{DisplayState, Focus, Marker, Song},
        either::Either4,
        ext::command::CommandChain,
    },
    arrayvec::ArrayVec,
    bumpalo::Bump,
    enum_map::{Enum, EnumMap},
    std::{cmp::Ordering, io, marker::Unpin, ptr},
    tokio::io::AsyncWriteExt,
};

#[derive(Clone, Copy, Debug, Enum, PartialEq)]
pub enum Damage {
    Draw(Focus, Marker),
    Remove(Focus, Marker),
    MoveOffset(Focus),
    FullRedraw,
}
impl Damage {
    /// Rank damage by how much they will change.
    ///
    ///  - [Self::FullRedraw] is [Ordering::Greater] than [Self::MoveOffset]
    ///  - [Self::MoveOffset] is [Ordering::Greater] than [Self::Draw] and [Self::Remove]
    ///  - [Self::Draw] is [Ordering::Equal] to [Self::Remove]
    pub fn rank(&self, r: &Self) -> Ordering {
        const fn to_ranking(damage: &Damage) -> u8 {
            match damage {
                Damage::FullRedraw => 3,
                Damage::MoveOffset(_) => 2,
                Damage::Draw(..) | Damage::Remove(..) => 1,
            }
        }

        to_ranking(self).cmp(&to_ranking(r))
    }

    pub fn render(&self, old: &DisplayState<'_>, new: &DisplayState<'_>) -> impl CommandChain {
        match self {
            Self::Draw(focus, marker) => Either4::First(
                marker
                    .get(*focus, new)
                    .map(|index| new.render_line(*focus, index)),
            ),
            Self::Remove(focus, marker) => Either4::Second(
                marker
                    .get(*focus, old)
                    .map(|index| new.render_line(*focus, index)),
            ),
            Self::FullRedraw => Either4::Third(
                new.render_menu(Focus::Playlists)
                    .then(new.render_menu(Focus::Songs)),
            ),
            Self::MoveOffset(focus) => Either4::Fourth(new.render_menu(*focus)),
        }
    }
    pub const fn resolves(&self) -> &'static [Self] {
        match self {
            Self::FullRedraw => &[
                Self::MoveOffset(Focus::Playlists),
                Self::MoveOffset(Focus::Songs),
                Self::Remove(Focus::Playlists, Marker::Cursor),
                Self::Remove(Focus::Playlists, Marker::Selection),
                Self::Remove(Focus::Songs, Marker::Cursor),
                Self::Remove(Focus::Songs, Marker::Selection),
                Self::Draw(Focus::Playlists, Marker::Cursor),
                Self::Draw(Focus::Playlists, Marker::Selection),
                Self::Draw(Focus::Songs, Marker::Cursor),
                Self::Draw(Focus::Songs, Marker::Selection),
            ],
            Self::MoveOffset(Focus::Playlists) => &[
                Self::Remove(Focus::Playlists, Marker::Cursor),
                Self::Remove(Focus::Playlists, Marker::Selection),
                Self::Draw(Focus::Playlists, Marker::Cursor),
                Self::Draw(Focus::Playlists, Marker::Selection),
            ],
            Self::MoveOffset(Focus::Songs) => &[
                Self::Remove(Focus::Songs, Marker::Cursor),
                Self::Remove(Focus::Songs, Marker::Selection),
                Self::Draw(Focus::Songs, Marker::Cursor),
                Self::Draw(Focus::Songs, Marker::Selection),
            ],
            _ => &[],
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

#[derive(Clone, Copy, Debug)]
pub struct DamageList<'a> {
    list: EnumMap<Damage, bool>,
    old: DisplayState<'a>,
    new: DisplayState<'a>,
}
impl CommandChain for DamageList<'_> {
    async fn execute<W>(self, alloc: &Bump, out: &mut W) -> Result<(), io::Error>
    where
        W: AsyncWriteExt + Unpin,
    {
        let mut damages = self
            .list
            .into_iter()
            .filter(|(_, enabled)| *enabled)
            .map(|(damage, _)| damage)
            .collect::<ArrayVec<Damage, { Damage::LENGTH }>>();
        damages.sort_by(|l, r| l.rank(r));

        while let Some(damage) = damages.pop() {
            damage
                .render(&self.old, &self.new)
                .execute(alloc, out)
                .await?;
            let resolutions = damage.resolves();
            damages.retain(|damage| !resolutions.contains(damage));
        }

        Ok(())
    }
}
