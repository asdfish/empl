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
    cfg_if::cfg_if,
    const_format::{
        self as cfmt,
        marker_traits::{FormatMarker, IsNotStdKind},
        writec,
    },
    std::{
        borrow::Cow,
        error::Error,
        ffi::{CStr, OsStr},
        fmt::{self, Display, Formatter},
        path::{Path, PathBuf},
    },
};

/// # Safety
///
/// - No other threads can modify environment variables.
unsafe fn get_env<'a, 'b>(var: &'a CStr) -> Result<Cow<'b, OsStr>, UnknownEnvVarError<'a>> {
    cfg_if! {
        if #[cfg(windows)] {
            // On windows you need to allocate so we can just use the standard library.
            std::env::var_os(unsafe { OsStr::from_encoded_bytes_unchecked(var.to_bytes()) }).map(Cow::Owned).ok_or(UnknownEnvVarError(var))
        } else {
            use libc::getenv;
            let val = unsafe { getenv(var.as_ptr()) };
            if val.is_null() {
                None
            } else {
                Some(unsafe { OsStr::from_encoded_bytes_unchecked(CStr::from_ptr(val).to_bytes()) })
            }
            .map(Cow::Borrowed)
            .ok_or(UnknownEnvVarError(var))
        }
    }
}

fn os_str_to_path<'a>(os_str: Cow<'a, OsStr>) -> Cow<'a, Path> {
    match os_str {
        Cow::Borrowed(os_str) => Cow::Borrowed(Path::new(os_str)),
        Cow::Owned(os_str) => Cow::Owned(PathBuf::from(os_str)),
    }
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
    pub const fn is_dynamic(&self) -> bool {
        !matches!(self, Self::Segment(_))
    }

    pub const fn size_hint(&self) -> Option<usize> {
        if let Self::Segment(segment) = self {
            Some(segment.len())
        } else {
            None
        }
    }

    /// # Safety
    ///
    /// This cannot be called when other threads are modifying environment variables or errno.
    pub unsafe fn to_path<'b>(self) -> Result<Cow<'b, Path>, GetPathSegmentError<'a>>
    where
        'a: 'b,
    {
        match self {
            #[cfg(unix)]
            Self::HomeDir => unsafe { get_env(c"HOME") }
                .map(os_str_to_path)
                .map_err(GetPathSegmentError::UnknownEnvVar)
                .or_else(|_| {
                    use {
                        errno::{Errno, errno, set_errno},
                        libc::{getpwuid, getuid},
                    };

                    // SAFETY: this never fails
                    let uid = unsafe { getuid() };

                    set_errno(Errno(0));

                    // SAFETY: we check errors
                    let pwd = unsafe { getpwuid(uid) };
                    // SAFETY: pointer should be convertible to a reference
                    if let Some(pwd) = unsafe { pwd.as_ref() }
                        && errno() != Errno(0)
                        && !pwd.pw_dir.is_null()
                    {
                        Ok(pwd.pw_dir)
                            // SAFETY: we check for null
                            .map(|home_dir| unsafe { CStr::from_ptr(home_dir) })
                            .map(CStr::to_bytes)
                            // SAFETY: [CStr]s should be converible into an [OsStr]
                            .map(|bytes| unsafe { OsStr::from_encoded_bytes_unchecked(bytes) })
                            .map(Path::new)
                            .map(Cow::Borrowed)
                    } else {
                        Err(GetPathSegmentError::ReadPwd(std::io::Error::last_os_error()))
                    }
                }),
            Self::EnvVar(var) => unsafe { get_env(var) }
                .map(os_str_to_path)
                .map_err(GetPathSegmentError::UnknownEnvVar),
            Self::Segment(segment) => Ok(Cow::Borrowed(Path::new(segment))),
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
            // Reading environment variables depends on the shell, and there is no way to reliably determine the shell so we default to posix shell.
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
    use {
        super::*, crate::tests::ENV_VAR_LOCK, arrayvec::ArrayVec, const_format::formatc, std::env,
    };

    #[test]
    fn path_segment_display() {
        #[cfg(unix)]
        {
            assert_eq!(formatc!("{}", PathSegment::HomeDir), "${HOME}");
        }
        assert_eq!(formatc!("{}", PathSegment::EnvVar(c"foo")), "${foo}");
        assert_eq!(formatc!("{}", PathSegment::Segment("bar")), "bar");
    }

    #[test]
    fn env_vars() {
        let _lock = ENV_VAR_LOCK.write().unwrap();
        unsafe { env::set_var("HOME", "/home/foo") };

        let mut homes = ArrayVec::<_, 3>::new();

        homes.push(
            unsafe { get_env(c"HOME") }
                .ok()
                .map(os_str_to_path)
                .unwrap(),
        );
        #[cfg(all(unix, not(miri)))]
        {
            homes.push(unsafe { PathSegment::HomeDir.to_path() }.unwrap());
        }
        homes.push(unsafe { PathSegment::EnvVar(c"HOME").to_path() }.unwrap());
        homes.into_iter().for_each(|home| {
            assert_eq!(home, Path::new("/home/foo"));
        });

        unsafe { env::remove_var("HOME") };

        [
            unsafe { get_env(c"HOME") }
                .map_err(GetPathSegmentError::UnknownEnvVar)
                .unwrap_err(),
            unsafe { PathSegment::EnvVar(c"HOME").to_path() }.unwrap_err(),
        ]
        .into_iter()
        .for_each(|error| {
            assert_eq!(
                error,
                GetPathSegmentError::UnknownEnvVar(UnknownEnvVarError(c"HOME"))
            )
        });

        #[cfg(all(unix, not(miri)))]
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
