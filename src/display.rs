use {
    bstr::BStr,
    getargs::Opt,
    std::fmt::{self, Formatter},
};

pub trait IntoDisplay {
    fn display(self) -> Display<Self>
    where
        Self: Sized,
        Display<Self>: fmt::Display,
    {
        Display(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Display<T>(T);
impl fmt::Display for Display<Opt<&[u8]>> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self.0 {
            Opt::Short(short) => write!(f, "-{}", char::from(short)),
            Opt::Long(long) => write!(f, "--{}", BStr::new(long)),
        }
    }
}

/// Implement display for byte options
impl IntoDisplay for Opt<&[u8]> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opt_display() {
        assert_eq!(
            Opt::<&'static [u8]>::Short(b'a').display().to_string(),
            "-a"
        );
        assert_eq!(
            Opt::<&'static [u8]>::Long(b"foo").display().to_string(),
            "--foo"
        );
    }
}
