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

#![cfg_attr(not(test), no_main)]

use {
    cli::{
        argv::Argv,
        parser::{Config, ParseCliArgumentsError},
    },
    std::{
        convert::identity,
        ffi::{c_char, c_int},
        io,
    },
};

pub mod cli;
pub mod config;
pub mod display;
#[cfg(test)]
mod tests {
    use std::sync::RwLock;

    /// Lock used to signal that environment variables are being written to during tests.
    pub static ENV_VAR_LOCK: RwLock<()> = RwLock::new(());
}

// SAFETY: Every c program has done this since the dawn of time.
#[cfg_attr(not(test), unsafe(no_mangle))]
extern "C" fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    Config::new(
        unsafe { Argv::new(argc, argv) }.skip(1),
        &mut io::stdout().lock(),
    )
    .map_err(|error| match error {
        ParseCliArgumentsError::PrintStdout(_) => todo!(),
        error => {
            eprintln!("{error}");
            1
        }
    })
    .and_then(|config| config.ok_or(0))
    .inspect(|config| println!("{config:?}"))
    .map_or_else(identity, |_| 0)
}
