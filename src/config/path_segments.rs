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
    crate::config::path_segment::{GetPathSegmentError, PathSegment},
    const_format::{
        self as cfmt,
        marker_traits::{FormatMarker, IsNotStdKind},
        try_, writec,
    },
    itertools::Itertools,
    std::path::{self, PathBuf},
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(transparent)]
pub struct PathSegments<'a>(&'a [PathSegment<'a>]);
impl<'a> PathSegments<'a> {
    pub const fn new(segments: &'a [PathSegment<'a>]) -> Self {
        Self(segments)
    }

    #[expect(unstable_name_collisions)]
    pub fn size_hint(&self) -> usize {
        self.0
            .iter()
            .map(|segment| segment.size_hint())
            .intersperse(Some(path::MAIN_SEPARATOR.len_utf8()))
            .flatten()
            .sum::<usize>()
    }

    /// # Safety
    ///
    /// See [PathSeparator::to_path]'s section on safety.
    pub unsafe fn to_path_buf(&self) -> Result<PathBuf, GetPathSegmentError<'a>> {
        self.0.iter().try_fold(
            PathBuf::with_capacity(self.size_hint()),
            |mut accum, segment| {
                unsafe { segment.to_path() }
                    .map(|segment| accum.push(segment))
                    .map(|_| accum)
            },
        )
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
                try_!(writec!(f, "{}", path::MAIN_SEPARATOR));
            }

            i += 1;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::tests::ENV_VAR_LOCK, const_format::formatc};

    #[test]
    fn path_segments_display() {
        let _lock = ENV_VAR_LOCK.read().unwrap();

        macro_rules! test_path_segments_display {
            ($segments:expr, $output:expr $(,)?) => {
                $segments.0.iter().for_each(|segment| {
                    assert!(
                        !segment.is_dynamic(),
                        "this test can only use static path segments"
                    )
                });

                let display = formatc!("{}", $segments);
                assert_eq!(display, $output);
                assert_eq!(
                    unsafe { $segments.to_path_buf() }
                        .unwrap()
                        .display()
                        .to_string(),
                    display
                );
            };
        }

        test_path_segments_display!(PathSegments(&[]), "");
        test_path_segments_display!(PathSegments(&[PathSegment::Segment("foo")]), "foo");
        test_path_segments_display!(
            PathSegments(&[PathSegment::Segment("foo"), PathSegment::Segment("bar")]),
            format!("foo{}bar", path::MAIN_SEPARATOR),
        );
    }
}
