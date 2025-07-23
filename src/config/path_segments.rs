// empl - Extensible Music PLayer
// Copyright (C) 2025  Andrew Chi

// This file is part of empl.

// empl is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// empl is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with empl.  If not, see <http://www.gnu.org/licenses/>.

pub mod choice;

use {
    crate::config::path_segment::PathSegment,
    const_format::{
        self as cfmt,
        marker_traits::{FormatMarker, IsNotStdKind},
        try_, writec,
    },
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(transparent)]
pub struct PathSegments<'a>(&'a [PathSegment<'a>]);
impl<'a> PathSegments<'a> {
    pub const fn new(segments: &'a [PathSegment<'a>]) -> Self {
        Self(segments)
    }
}
impl FormatMarker for PathSegments<'_> {
    type Kind = IsNotStdKind;
    type This = Self;
}
impl PathSegments<'_> {
    pub const fn const_display_fmt(&self, f: &mut cfmt::Formatter<'_>) -> Result<(), cfmt::Error> {
        let mut i = 0;
        while i < self.0.len() {
            try_!(writec!(f, "{}", self.0[i]));
            if i < self.0.len() - 1 {
                try_!(writec!(f, "/"));
            }

            i += 1;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use {super::*, const_format::formatc};

    #[test]
    fn path_segments_display() {
        assert_eq!(formatc!("{}", PathSegments(&[])), "");
        assert_eq!(
            formatc!("{}", PathSegments(&[PathSegment::Segment("foo")])),
            "foo"
        );
        assert_eq!(
            formatc!(
                "{}",
                PathSegments(&[PathSegment::Segment("foo"), PathSegment::Segment("bar")])
            ),
            "foo/bar"
        );
    }
}
