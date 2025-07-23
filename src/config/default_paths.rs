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

use {crate::config::path_segment::PathSegment, cfg_if::cfg_if};

cfg_if! {
    if #[cfg(windows)] {
        const DEFAULT_PATHS: &[&[PathSegment]] = &[&[
            PathSegment::EnvVar(c"%APPDATA%"),
            PathSegment::Segment("empl"),
            PathSegment::Segment("config"),
            PathSegment::Segment("main.scm"),
        ]];
    } else if #[cfg(target_os = "macos")] {
        const DEFAULT_PATHS: &[&[PathSegment]] = &[&[
            PathSegment::HomeDir,
            PathSegment::Segment("Library"),
            PathSegment::Segment("Application Support"),
            PathSegment::Segment("empl"),
            PathSegment::Segment("main.scm"),
        ]];
    } else if #[cfg(unix)] {
        const DEFAULT_PATHS: &[&[PathSegment]] = &[
            &[
                PathSegment::EnvVar(c"XDG_CONFIG_HOME"),
                PathSegment::Segment("empl"),
                PathSegment::Segment("main.scm"),
            ],
            &[
                PathSegment::HomeDir,
                PathSegment::Segment(".config"),
                PathSegment::Segment("empl"),
                PathSegment::Segment("main.scm"),
            ]
        ];
    } else {
        compile_error!("unsupported platform");
    }
}
