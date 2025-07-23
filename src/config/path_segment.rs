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
    bstr::BStr,
    const_format::{
        self as cfmt,
        marker_traits::{FormatMarker, IsNotStdKind},
        writec,
    },
    libc::getenv,
    std::{
        error::Error,
        ffi::{CStr, OsStr},
        fmt::{self, Display, Formatter},
        path::Path,
    },
};

/// Get an environment variable in a way safer than [getenv] thanks to its type signature.
///
/// # Safety
///
/// - No other threads can modify environment variables.
unsafe fn get_env<'a, 'b>(var: &'a CStr) -> Result<&'b OsStr, UnknownEnvVarError<'a>> {
    let val = unsafe { getenv(var.as_ptr()) };
    if val.is_null() {
        None
    } else {
        Some(unsafe { OsStr::from_encoded_bytes_unchecked(CStr::from_ptr(val).to_bytes()) })
    }
    .ok_or(UnknownEnvVarError(var))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnknownEnvVarError<'a>(&'a CStr);
impl Display for UnknownEnvVarError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(
            f,
            "unknown environment variable `{}`",
            BStr::new(self.0.to_bytes())
        )
    }
}
impl Error for UnknownEnvVarError<'_> {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PathSegment<'a> {
    #[cfg(unix)]
    HomeDir,
    EnvVar(&'a CStr),
    Segment(&'a str),
}
impl<'a> PathSegment<'a> {
    /// # Safety
    ///
    /// This cannot be called when other threads are modifying environment variables or errno.
    pub unsafe fn to_path<'b>(self) -> Result<&'b Path, GetPathSegmentError<'a>>
    where
        'a: 'b,
    {
        match self {
            #[cfg(unix)]
            Self::HomeDir => unsafe { get_env(c"HOME") }
                .map(Path::new)
                .map_err(GetPathSegmentError::UnknownEnvVar)
                .or_else(|_| {
                    use libc::{__errno_location, getpwuid, getuid};

                    // SAFETY: this never fails
                    let uid = unsafe { getuid() };

                    // SAFETY: we don't read it yet
                    let errno = unsafe { __errno_location() };
                    // SAFETY: precondition is in contract
                    if let Some(errno) = unsafe { __errno_location().as_mut() } {
                        *errno = 0;
                    }

                    // SAFETY: we check errors
                    let pwd = unsafe { getpwuid(uid) };
                    // SAFETY: pointer should be convertible to a reference
                    if let Some(pwd) = unsafe { pwd.as_ref() }
                    // SAFETY: nothing should mutate errno in this seciton
                    && unsafe { errno.as_ref() }.map(|errno| *errno == 0).unwrap_or(true)
                        && !pwd.pw_dir.is_null()
                    {
                        Ok(pwd.pw_dir)
                            // SAFETY: we check for null
                            .map(|home_dir| unsafe { CStr::from_ptr(home_dir) })
                            .map(CStr::to_bytes)
                            // SAFETY: [CStr]s should be converible into an [OsStr]
                            .map(|bytes| unsafe { OsStr::from_encoded_bytes_unchecked(bytes) })
                            .map(Path::new)
                    } else {
                        Err(GetPathSegmentError::ReadPwd(std::io::Error::last_os_error()))
                    }
                }),
            Self::EnvVar(var) => unsafe { get_env(var) }
                .map(Path::new)
                .map_err(GetPathSegmentError::UnknownEnvVar),
            Self::Segment(segment) => Ok(Path::new(segment)),
        }
    }
}
impl FormatMarker for PathSegment<'_> {
    type Kind = IsNotStdKind;
    type This = Self;
}
impl PathSegment<'_> {
    /// # Panics
    ///
    ///  - This function sill panic if `Self::EnvVar(var)` is not a valid utf8 string.
    pub const fn const_display_fmt(&self, f: &mut cfmt::Formatter<'_>) -> Result<(), cfmt::Error> {
        match self {
            #[cfg(unix)]
            Self::HomeDir => writec!(f, "${{HOME}}"),
            Self::EnvVar(var) => match str::from_utf8(var.to_bytes()) {
                Ok(var) => writec!(f, "${{{}}}", var),
                Err(_) => panic!(),
            },
            Self::Segment(segment) => writec!(f, "{}", segment),
        }
    }
}

#[derive(Debug)]
pub enum GetPathSegmentError<'a> {
    #[cfg(unix)]
    ReadPwd(std::io::Error),
    UnknownEnvVar(UnknownEnvVarError<'a>),
}
impl PartialEq for GetPathSegmentError<'_> {
    fn eq(&self, r: &Self) -> bool {
        #[allow(unreachable_patterns)]
        match (self, r) {
            #[cfg(unix)]
            (Self::ReadPwd(l), Self::ReadPwd(r)) => l.kind() == r.kind(),
            (Self::UnknownEnvVar(l), Self::UnknownEnvVar(r)) => l == r,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, const_format::formatc};

    #[test]
    fn path_segment_display() {
        #[cfg(unix)]
        {
            assert_eq!(formatc!("{}", PathSegment::HomeDir), "${HOME}");
        }
        assert_eq!(formatc!("{}", PathSegment::EnvVar(c"foo")), "${foo}");
        assert_eq!(formatc!("{}", PathSegment::Segment("bar")), "bar");
    }

    /// # Safety
    ///
    /// No other test can modify environment variables since those can be ran in parallel.
    #[test]
    fn env_vars() {
        use arrayvec::ArrayVec;
        use std::env;

        unsafe { env::set_var("HOME", "/home/foo") };

        let mut homes = ArrayVec::<_, 3>::new();

        homes.push(unsafe { get_env(c"HOME") }.ok().map(Path::new));
        #[cfg(unix)]
        {
            homes.push(unsafe { PathSegment::HomeDir.to_path() }.ok());
        }
        homes.push(unsafe { PathSegment::EnvVar(c"HOME").to_path() }.ok());
        homes.into_iter().enumerate().for_each(|(i, home)| {
            println!("test 1/{i}");
            assert_eq!(home, Some(Path::new("/home/foo")));
        });

        unsafe { env::remove_var("HOME") };

        [
            unsafe { get_env(c"HOME") }
                .err()
                .map(GetPathSegmentError::UnknownEnvVar),
            unsafe { PathSegment::EnvVar(c"HOME").to_path() }.err(),
        ]
        .into_iter()
        .enumerate()
        .for_each(|(i, error)| {
            println!("test 2/{i}");
            assert_eq!(
                error,
                Some(GetPathSegmentError::UnknownEnvVar(UnknownEnvVarError(
                    c"HOME"
                )))
            )
        });

        #[cfg(unix)]
        {
            assert_ne!(
                unsafe { PathSegment::HomeDir.to_path() },
                Err(GetPathSegmentError::UnknownEnvVar(UnknownEnvVarError(
                    c"HOME"
                )))
            );
        }
    }
}
