use {
    crate::{
        command::PrintPadded,
        config::{Config, Playlists, SelectedConfig},
        ext::{
            colors::ColorsExt,
            command::{CommandChain, CommandExt},
            iterator::IteratorExt,
        },
        tasks::display::damage::{Damage, DamageList},
    },
    crossterm::{cursor::MoveTo, style::SetColors, terminal},
    enum_map::{Enum, EnumMap},
    std::num::{NonZeroU16, NonZeroUsize},
};

#[derive(Clone, Copy, Debug, Default, Enum, PartialEq)]
pub enum Focus {
    #[default]
    Playlists,
    Songs,
}

#[derive(Clone, Copy, Debug, Enum, PartialEq)]
pub enum Marker {
    Cursor,
    Selection,
}
impl Marker {
    pub fn get(&self, focus: Focus, state: &DisplayState) -> u16 {
        match (self, focus) {
            (Self::Cursor, focus) => state.cursors[focus],
            (Self::Selection, Focus::Playlists) => state.selected_menu,
            (Self::Selection, Focus::Songs) => state.selected_song.index,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplayState<'a> {
    pub focus: Focus,
    pub cursors: EnumMap<Focus, u16>,
    pub offsets: EnumMap<Focus, u16>,
    pub selected_menu: u16,
    pub selected_song: Song,
    pub terminal_area: Option<Area>,
    pub(super) playlists: &'a Playlists,
}
impl<'a> DisplayState<'a> {
    pub fn new(playlists: &'a Playlists) -> Self {
        Self {
            focus: Focus::Playlists,
            cursors: EnumMap::default(),
            offsets: EnumMap::default(),
            selected_menu: 0,
            selected_song: Song::default(),
            terminal_area: terminal::size().ok().and_then(|(width, height)| {
                Some(Area {
                    width: NonZeroU16::new(width)?,
                    height: NonZeroU16::new(height)?,
                })
            }),
            playlists,
        }
    }

    pub fn playlists(&self) -> &'a Playlists {
        self.playlists
    }

    fn get(&self, focus: Focus, index: u16) -> Option<&str> {
        match focus {
            Focus::Playlists => self
                .playlists
                .get(usize::from(index))
                .map(|(item, _)| item.as_str()),
            Focus::Songs => self
                .playlists
                .get(usize::from(self.selected_menu))
                .map(|(_, playlist)| playlist)
                .and_then(|playlist| playlist.get(usize::from(index)))
                .map(|(item, _)| item)
                .map(|item| item.as_str()),
        }
    }
    pub fn len(&self, focus: Focus) -> Option<NonZeroUsize> {
        match focus {
            Focus::Playlists => Some(self.playlists.len()),
            Focus::Songs => self
                .playlists
                .get(usize::from(self.selected_menu))
                .map(|(_, playlist)| playlist.len()),
        }
    }

    fn check_offset(&mut self) {
        if let Some(len) = self.len(self.focus) {
            if usize::from(self.cursors[self.focus]) > len.get() - 1 {
                self.cursors[self.focus] = u16::try_from(len.get() - 1).unwrap_or(u16::MAX);
            }
        }

        if self.offsets[self.focus] > self.cursors[self.focus] {
            self.offsets[self.focus] = self.cursors[self.focus];
        }

        if let Some(height) = self.terminal_area.map(|Area { height, .. }| height) {
            if self.cursors[self.focus] >= self.offsets[self.focus].saturating_add(height.get()) {
                self.offsets[self.focus] =
                    self.cursors[self.focus].saturating_sub(height.get()) + 1;
            }
        }
    }

    fn row(&self, focus: Focus) -> Option<Row> {
        match (focus, self.terminal_area) {
            (Focus::Playlists, Some(Area { width, .. })) => width
                .get()
                .checked_div(2)
                .and_then(NonZeroU16::new)
                .map(|width| Row { x: 0, width }),
            (Focus::Songs, Some(Area { width, .. })) => {
                let last_width = self
                    .row(Focus::Playlists)
                    .map(|Row { width, .. }| width)
                    .map(NonZeroU16::get)
                    .unwrap_or_default();

                NonZeroU16::new(width.get().saturating_sub(last_width)).map(|width| Row {
                    x: last_width,
                    width,
                })
            }
            _ => None,
        }
    }

    pub fn render_line(&self, focus: Focus, index: u16) -> impl CommandChain {
        index
            .checked_sub(self.offsets[focus])
            .and_then(|y| self.row(focus).map(|Row { x, width }| (x, y, width)))
            .map(|(x, y, width)| {
                let mut colors = SelectedConfig::MENU_COLORS;
                if self.focus == focus && index == Marker::Cursor.get(focus, self) {
                    colors.join(&SelectedConfig::CURSOR_COLORS);
                }
                if ((focus == Focus::Songs && self.selected_menu == self.selected_song.playlist)
                    || focus == Focus::Playlists)
                    && index == Marker::Selection.get(focus, self)
                {
                    colors.join(&SelectedConfig::SELECTION_COLORS);
                }

                SetColors(colors).adapt().then(MoveTo(x, y).adapt()).then(
                    PrintPadded {
                        text: self.get(focus, index).unwrap_or(""),
                        padding: ' ',
                        width: usize::from(width.get()),
                    }
                    .adapt(),
                )
            })
    }

    pub fn render_menu(&self, focus: Focus) -> impl CommandChain {
        self.terminal_area.map(move |Area { height, .. }| {
            (self.offsets[focus]..self.offsets[focus] + height.get())
                .map(move |index| self.render_line(focus, index))
                .adapt()
        })
    }

    pub fn write<F>(&mut self, operation: F) -> DamageList<'a>
    where
        F: FnOnce(&mut Self),
    {
        let old = *self;
        operation(self);
        self.check_offset();

        if old.eq(self) {
            DamageList::new(EnumMap::default(), old, *self)
        } else {
            DamageList::new(
                EnumMap::from_fn(|damage: Damage| damage.predicate(&old, self)),
                old,
                *self,
            )
        }
    }

    pub fn visible(&self, focus: Focus, index: u16) -> bool {
        index >= self.offsets[focus]
            && index
                < self.offsets[focus]
                    + self
                        .terminal_area
                        .map(|Area { width, .. }| width)
                        .map(NonZeroU16::get)
                        .unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Area {
    pub width: NonZeroU16,
    pub height: NonZeroU16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Row {
    x: u16,
    width: NonZeroU16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Song {
    pub playlist: u16,
    pub index: u16,
}

#[cfg(test)]
mod tests {
    use {super::*, nonempty_collections::NEVec, std::path::PathBuf};

    #[test]
    fn display_state_is_visible() {
        let display_state = DisplayState {
            focus: Focus::Playlists,
            offsets: EnumMap::from_fn(|_| 0),
            terminal_area: const {
                Some(Area {
                    width: NonZeroU16::new(1).unwrap(),
                    height: NonZeroU16::new(1).unwrap(),
                })
            },
            cursors: EnumMap::default(),
            selected_song: None,
            playlists: &NEVec::new((
                String::from(""),
                NEVec::new((String::from(""), PathBuf::new())),
            )),
        };

        assert_eq!(display_state.visible(Focus::Playlists, 0), true);
        assert_eq!(display_state.visible(Focus::Playlists, 1), false);
    }
}
