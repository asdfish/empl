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
    bindgen::BindgenError,
    std::{
        env,
        error::Error,
        fmt::{self, Display, Formatter},
        io::{self, Write, stdout},
        path::{self, Path, PathBuf},
        process::{Command, ExitCode},
        str,
    },
};

const GUILE_CONFIG_COMMANDS: &[&str] = &["guile-config", "guile-config-3.0"];
const _: () = assert!(!GUILE_CONFIG_COMMANDS.is_empty());

fn guile_config(subcommand: &str) -> Result<Vec<u8>, io::Error> {
    let mut last_error = None;

    for command in GUILE_CONFIG_COMMANDS {
        match Command::new(command)
            .arg(subcommand)
            .output()
            .map(|output| output.stdout)
        {
            Ok(output) => return Ok(output),
            Err(error) => {
                last_error = Some(error);
            }
        }
    }

    Err(last_error.unwrap())
}

fn main() -> ExitCode {
    let mut stdout = stdout().lock();
    stdout
        .write_all(
            b"cargo:rerun-if-changed=build.rs
cargo:rerun-if-changed=./src/reexports.c
cargo:rerun-if-changed=./src/reexports.h\n",
        )
        .and_then(|_| guile_config("link"))
        .and_then(|linker_args| {
            linker_args
                .split(u8::is_ascii_whitespace)
                .filter(|linker_arg| !linker_arg.is_empty())
                .try_for_each(|linker_arg| {
                    stdout
                        .write_all(b"cargo:rustc-link-arg=")
                        .and_then(|_| stdout.write_all(linker_arg))
                        .and_then(|_| stdout.write_all(b"\n"))
                })
        })
        .and_then(|_| stdout.flush())
        .and_then(|_| guile_config("compile"))
        .map(|compile_args| {
            let mut libguile = None;

            let (mut cc, bindgen) = compile_args
                .split(u8::is_ascii_whitespace)
                .flat_map(str::from_utf8)
                .filter(|arg| !arg.is_empty())
                .fold(
                    (cc::Build::new(), bindgen::Builder::default()),
                    |(mut cc, bindgen), compile_arg| {
                        if let Some(include_dir) = compile_arg.strip_prefix("-I") {
                            let mut path = include_dir.to_string();
                            if !path.ends_with(path::MAIN_SEPARATOR) {
                                path.push(path::MAIN_SEPARATOR);
                            }
                            path.push_str("libguile.h");

                            if Path::new(&path).is_file() {
                                libguile = Some(path);
                            }
                        }

                        cc.flag(compile_arg);
                        (cc, bindgen.clang_arg(compile_arg))
                    },
                );

            cc.file("src/reexports.c").compile("reexports");
            bindgen
                .header(libguile.expect("failed to find `libguile.h`"))
                .header_contents("src/reexports.h", include_str!("src/reexports.h"))
                .wrap_static_fns(true)
        })
        .map_err(BuildError::Io)
        .and_then(|bindings| bindings.generate().map_err(BuildError::Bindgen))
        .and_then(|bindings| {
            const ENV_VAR: &str = "OUT_DIR";

            env::var_os(ENV_VAR)
                .ok_or(BuildError::EnvVar(ENV_VAR))
                .map(PathBuf::from)
                .map(|path| path.join("libguile.rs"))
                .and_then(|path| bindings.write_to_file(path).map_err(BuildError::Io))
        })
        .map_or_else(
            |error| {
                eprintln!("{error}");
                ExitCode::FAILURE
            },
            |_| ExitCode::SUCCESS,
        )
}

#[derive(Debug)]
pub enum BuildError {
    Bindgen(BindgenError),
    Io(io::Error),
    EnvVar(&'static str),
}
impl Display for BuildError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Bindgen(e) => e.fmt(f),
            Self::Io(e) => e.fmt(f),
            Self::EnvVar(var) => write!(f, "failed to find environment variable `{var}`"),
        }
    }
}
impl Error for BuildError {}
