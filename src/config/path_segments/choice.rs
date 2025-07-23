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

use {
    crate::config::path_segments::PathSegments,
    const_format::{
        self as cfmt, Formatter,
        marker_traits::{FormatMarker, IsNotStdKind},
        try_, writec,
    },
};

#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(transparent)]
pub struct Choice<'a>(&'a [PathSegments<'a>]);
impl FormatMarker for Choice<'_> {
    type Kind = IsNotStdKind;
    type This = Self;
}
impl<'a> Choice<'a> {
    /// Create a new choice list.
    ///
    /// This will return [None] if [<[_]>::len] is less than 1.
    pub const fn new(items: &'a [PathSegments<'a>]) -> Option<Self> {
        if items.len().is_empty() {
            None
        } else {
            Some(Self(items))
        }
    }
}
impl Choice<'_> {
    pub const fn const_display_fmt(&self, f: &mut Formatter<'_>) -> Result<(), cfmt::Error> {
        if self.0.len() == 1 {
            try_!(writec!(f, "`{}`", self.0[0]));
        } else {
            try_!(writec!(f, "either "));

            let mut i = 0;
            while i < self.0.len() {
                try_!(writec!(f, "`{}`", self.0[i]));

                if i == self.0.len() - 2 {
                    try_!(writec!(f, ", or "));
                } else if i != self.0.len() - 1 {
                    try_!(writec!(f, ", "));
                }

                i += 1;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*, crate::config::path_segment::PathSegment, arrayvec::ArrayVec,
        const_format::formatc,
    };

    #[test]
    fn choice_length_requirement() {
        (0..1)
            .map(|len| {
                (0..len)
                    .map(|_| PathSegments::default())
                    .collect::<ArrayVec<_, 2>>()
            })
            .for_each(|segments| {
                assert_eq!(Choice::new(&segments), None);
            })
    }

    #[test]
    fn choice_display() {
        assert_eq!(
            formatc!(
                "{}",
                Choice::new(&[PathSegments(&[PathSegment::Segment("foo"),]),]).unwrap()
            ),
            "`foo`"
        );
        assert_eq!(
            formatc!(
                "{}",
                Choice::new(&[
                    PathSegments(&[PathSegment::Segment("foo"),]),
                    PathSegments(&[PathSegment::Segment("bar"),]),
                ])
                .unwrap()
            ),
            "either `foo`, or `bar`"
        );
        assert_eq!(
            formatc!(
                "{}",
                Choice::new(&[
                    PathSegments(&[PathSegment::Segment("foo"),]),
                    PathSegments(&[PathSegment::Segment("bar"),]),
                    PathSegments(&[PathSegment::Segment("baz"),]),
                ])
                .unwrap()
            ),
            "either `foo`, `bar`, or `baz`"
        );
    }
}
