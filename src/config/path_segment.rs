use {
    bstr::BStr,
    libc::getenv,
    std::{
        error::Error,
        ffi::{CStr, OsStr},
        fmt::{self, Display, Formatter},
        io,
        path::Path,
    },
};

/// Get an environment variable in a way safer than [getenv] thanks to its type signature.
///
/// # Safety
///
/// - No other threads can modify environment variables.
#[cfg(unix)]
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
impl Display for PathSegment<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            #[cfg(unix)]
            Self::HomeDir => "${HOME}".fmt(f),
            Self::EnvVar(var) => write!(f, "${{{}}}", BStr::new(var.to_bytes())),
            Self::Segment(segment) => segment.fmt(f),
        }
    }
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
                        Err(GetPathSegmentError::ReadPwd(io::Error::last_os_error()))
                    }
                }),
            Self::EnvVar(var) => unsafe { get_env(var) }
                .map(Path::new)
                .map_err(GetPathSegmentError::UnknownEnvVar),
            Self::Segment(segment) => Ok(Path::new(segment)),
        }
    }
}

#[derive(Debug)]
pub enum GetPathSegmentError<'a> {
    #[cfg(unix)]
    ReadPwd(io::Error),
    UnknownEnvVar(UnknownEnvVarError<'a>),
}
impl PartialEq for GetPathSegmentError<'_> {
    fn eq(&self, r: &Self) -> bool {
        match (self, r) {
            (Self::ReadPwd(l), Self::ReadPwd(r)) => l.kind() == r.kind(),
            (Self::UnknownEnvVar(l), Self::UnknownEnvVar(r)) => l == r,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_segment_display() {
        assert_eq!(PathSegment::HomeDir.to_string(), "${HOME}");
        assert_eq!(PathSegment::EnvVar(c"foo").to_string(), "${foo}");
        assert_eq!(PathSegment::Segment("bar").to_string(), "bar");
    }

    /// # Safety
    ///
    /// No other test can modify environment variables since those can be ran in parallel.
    #[test]
    fn env_vars() {
        use arrayvec::ArrayVec;
        use libc::{setenv, unsetenv};

        unsafe { setenv(c"HOME".as_ptr(), c"/home/foo".as_ptr(), 1) };

        let mut homes = ArrayVec::<_, 3>::new();

        homes.push(unsafe { get_env(c"HOME") }.ok().map(Path::new));
        #[cfg(unix)]
        {
            homes.push(unsafe { PathSegment::HomeDir.to_path() }.ok());
        }
        homes.push(unsafe { PathSegment::EnvVar(c"HOME").to_path() }.ok());
        homes
            .into_iter()
            .for_each(|home| assert_eq!(home, Some(Path::new("/home/foo"))));

        unsafe { unsetenv(c"HOME".as_ptr()) };

        [
            unsafe { get_env(c"HOME") }
                .err()
                .map(GetPathSegmentError::UnknownEnvVar),
            unsafe { PathSegment::EnvVar(c"HOME").to_path() }.err(),
        ]
        .into_iter()
        .for_each(|error| {
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
