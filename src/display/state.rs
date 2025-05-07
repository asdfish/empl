use {
    crate::{
        command::PrintPadded,
        config::{Config, Playlists, SelectedConfig},
        ext::{
            colors::ColorsExt,
            command::{CommandChain, CommandExt},
            iterator::IteratorExt,
        },
    },
    crossterm::{cursor::MoveTo, style::SetColors, terminal},
    enum_map::{Enum, EnumMap},
    std::num::NonZeroU16,
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
    pub fn get(&self, focus: Focus, state: &DisplayState) -> Option<u16> {
        match (self, focus) {
            (Self::Cursor, focus) => Some(state.cursors[focus]),
            (Self::Selection, Focus::Playlists) => {
                state.selected_song.map(|Song { playlist, .. }| playlist)
            }
            (Self::Selection, Focus::Songs) => state.selected_song.map(|Song { index, .. }| index),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplayState<'a> {
    pub focus: Focus,
    pub cursors: EnumMap<Focus, u16>,
    pub offsets: EnumMap<Focus, u16>,
    pub selected_song: Option<Song>,
    pub terminal_area: Option<Area>,
    pub playlists: &'a Playlists,
}
impl<'a> DisplayState<'a> {
    pub fn new(playlists: &'a Playlists) -> Self {
        Self {
            focus: Focus::Playlists,
            cursors: EnumMap::default(),
            offsets: EnumMap::default(),
            selected_song: None,
            terminal_area: terminal::size().ok().and_then(|(width, height)| {
                Some(Area {
                    width: NonZeroU16::new(width)?,
                    height: NonZeroU16::new(height)?,
                })
            }),
            playlists,
        }
    }

    fn get(&self, focus: Focus, index: u16) -> Option<&str> {
        match focus {
            Focus::Playlists => self
                .playlists
                .get(usize::from(index))
                .map(|(item, _)| item.as_str()),
            Focus::Songs => self
                .selected_song
                .map(|Song { playlist, .. }| playlist)
                .and_then(|playlist| self.playlists.get(usize::from(playlist)))
                .map(|(_, playlist)| playlist)
                .and_then(|playlist| playlist.get(usize::from(index)))
                .map(|(item, _)| item)
                .map(|item| item.as_str()),
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
                if Some(index) == Marker::Cursor.get(focus, self) {
                    colors.join(&SelectedConfig::CURSOR_COLORS);
                }
                if Some(index) == Marker::Selection.get(focus, self) {
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

#[derive(Clone, Debug)]
pub struct DisplayStateWriter<'a>(DisplayState<'a>);
impl<'a> DisplayStateWriter<'a> {
    pub fn new(playlists: &'a Playlists) -> Self
    {
        Self(DisplayState::new(playlists))
    }

    pub fn write<F>(&mut self, operation: F)
    where F: FnOnce(DisplayState<'a>) -> DisplayState<'a> {
        let old = self.0;
        self.0 = operation(self.0);
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
    use super::*;

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
            ..Default::default()
        };

        assert_eq!(display_state.is_visible(0), true);
        assert_eq!(display_state.is_visible(1), false);
    }
}
