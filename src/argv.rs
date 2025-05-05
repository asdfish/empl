use std::{
    error::Error,
    ffi::{c_char, c_int, CStr},
    fmt::{self, Display, Formatter},
    num::TryFromIntError,
    slice,
    str::Utf8Error,
};

/// # Invariants:
///
///  - All pointers are not null and are valid utf-8.
#[repr(transparent)]
pub struct Argv(&'static [*const c_char]);
impl Argv {
    /// # Safety
    ///
    ///   - `argv` must be safe to read (can be null)
    ///   - `argc` must be accurate
    pub unsafe fn new(argc: c_int, argv: *const *const c_char) -> Result<Self, ArgvError> {
        if argv.is_null() {
            return Err(ArgvError::Null);
        }

        let argc = match usize::try_from(argc) {
            Ok(argc) => argc,
            Err(err) => return Err(ArgvError::InvalidArgc(err)),
        };
        let argv = unsafe { slice::from_raw_parts(argv, argc) };

        Ok(Self(argv))
    }
}
impl Iterator for Argv {
    type Item = Result<&'static str, ArgError>;
    
    fn next(&mut self) -> Option<Result<&'static str, ArgError>> {
        match self.0 {
            [car, ..] if car.is_null() => Some(Err(ArgError::Null)),
            [car, cdr @ ..] => {
                self.0 = cdr;
                let arg = unsafe { CStr::from_ptr(*car) };
                Some(arg.to_str().map_err(|err| ArgError::Utf8(arg, err)))
            },
            [] => None,
        }
    }
}

#[derive(Debug)]
pub enum ArgError {
    Null,
    Utf8(&'static CStr, Utf8Error),
}
impl Display for ArgError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Null => f.write_str("argument is null"),
            Self::Utf8(a, e) => write!(f, "argument `{:?}` contains invalid utf8: {}", a, e),
        }
    }
}
impl Error for ArgError {}

#[derive(Debug)]
pub enum ArgvError {
    InvalidArgc(TryFromIntError),
    Null,
}
impl Display for ArgvError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::InvalidArgc(e) => write!(f, "`argc` is invalid: {}", e),
            Self::Null => f.write_str("`argv` is null"),
        }
    }
}
impl Error for ArgvError {}
