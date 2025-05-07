use {
    crate::{
        command::PrintPadded,
        config::{Config, SelectedConfig},
        ext::{
            colors::ColorsExt,
            command::{CommandChain, CommandExt},
            iterator::IteratorExt,
        },
    },
    crossterm::{cursor::MoveTo, style::SetColors, terminal},
    enum_iterator::Sequence,
    enum_map::{Enum, EnumMap},
    std::{
        iter,
        num::NonZeroU16,
    },
};

#[derive(Clone, Copy, Debug, Default, Enum, PartialEq, Sequence)]
pub enum Focus {
    #[default]
    Playlists,
    Songs,
}

#[derive(Clone, Copy, Debug, Enum, PartialEq, Sequence)]
pub enum Marker {
    Cursor,
    Selection,
}
impl Marker {
    pub fn get(&self, focus: Focus, state: &DisplayState) -> u16 {
        match (self, focus) {
            (Self::Cursor, focus) => state.cursors[focus],
            (Self::Selection, Focus::Playlists) => state.selected_song.playlist,
            (Self::Selection, Focus::Songs) => state.selected_song.index,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DisplayState {
    pub focus: Focus,
    pub cursors: EnumMap<Focus, u16>,
    pub offsets: EnumMap<Focus, u16>,
    pub selected_song: Song,
    pub terminal_area: Option<Area>,
}
impl DisplayState {
    pub fn new() -> Self {
        Self {
            terminal_area: terminal::size().ok().and_then(|(l, r)| {
                Some(Area {
                    width: NonZeroU16::new(l)?,
                    height: NonZeroU16::new(r)?,
                })
            }),
            ..Default::default()
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

    pub fn render_line<S>(&self, focus: Focus, item: Option<S>, index: u16) -> impl CommandChain
    where S: AsRef<str> + From<&'static str> {
        index
            .checked_sub(self.offsets[focus])
            .and_then(|y| self.row(focus).map(|Row { x, width }| (x, y, width)))
            .map(|(x, y, width)| {
                let mut colors = SelectedConfig::MENU_COLORS;
                if index == Marker::Cursor.get(focus, self) {
                    colors.join(&SelectedConfig::CURSOR_COLORS);
                }
                if index == Marker::Selection.get(focus, self) {
                    colors.join(&SelectedConfig::SELECTION_COLORS);
                }

                SetColors(colors)
                    .adapt()
                    .then(MoveTo(x, y).adapt())
                    .then(PrintPadded { text: item.unwrap_or_else(|| S::from("")), padding: ' ', width: usize::from(width.get()) }.adapt())
            })
    }

    pub fn render_menu<I, S>(&self, focus: Focus, items: I) -> impl CommandChain
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str> + From<&'static str>,
    {
        self.terminal_area.map(move |Area { height, .. }| {
            items
                .into_iter()
                .map(Some)
                .chain(iter::repeat_with(|| None))
                .take(usize::from(height.get()))
                .zip(0..)
                .map_command(move |(item, index)| self.render_line(focus, item, index))
        })

        // self.row(focus)
        //     .and_then(|Row { x, width }| self.terminal_area.map(move |Area { height, .. }| (x, width, height)))
        //     .map(|(x, width, height)| {
        //         items
        //             .into_iter()
        //             .map(Some)
        //             .chain(iter::repeat_with(|| None))
        //             .take(usize::from(height.get()))
        //             .zip(0..)
        //             .skip(usize::from(self.offsets[focus]))
        //             .zip(0..)
        //             .map_command(move |((item, index), y)| {
        //                 let mut colors = SelectedConfig::MENU_COLORS;
        //                 if index == Marker::Cursor.get(focus, self) {
        //                     colors.join(&SelectedConfig::CURSOR_COLORS);
        //                 }
        //                 if index == Marker::Selection.get(focus, self) {
        //                     colors.join(&SelectedConfig::SELECTION_COLORS);
        //                 }

        //                 SetColors(colors)
        //                     .adapt()
        //                     .then(MoveTo(x, y).adapt())
        //                     .then(PrintPadded { text: item.unwrap_or_else(|| S::from("")), padding: ' ', width: usize::from(width.get()) }.adapt())
        //             })
        //     })
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
