use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Flag<'a> {
    /// Flags that start with `-` or are chained
    Short(char),
    /// Flags that start with `--`
    Long(&'a str),
}
/// The discriminant type of [Flag]
#[derive(Clone, Copy, Debug, PartialEq)]
enum FlagType {
    Short,
    Long,
}
impl From<Flag<'_>> for FlagType {
    fn from(flag: Flag<'_>) -> FlagType {
        match flag {
            Flag::Short(_) => FlagType::Short,
            Flag::Long(_) => FlagType::Long,
        }
    }
}

/// Iterator for [Flag]s
///
/// # Examples
///
/// ```
/// # use empl::argument::{FlagIter, NonFlagError};
/// [
///     ("--", Err(NonFlagError("--"))),
///     ("-", Err(NonFlagError("-"))),
///     ("-a", Ok(FlagIter::Short("a"))),
///     (
///         "--help",
///         Ok(FlagIter::Long {
///             flag: Some("help"),
///             value: None,
///         }),
///     ),
///     (
///         "--foo=bar",
///         Ok(FlagIter::Long {
///             flag: Some("foo"),
///             value: Some("bar"),
///         }),
///     ),
/// ]
/// .into_iter()
/// .for_each(|(l, r)| assert_eq!(FlagIter::try_from(l), r))
/// ```
///
/// ```
/// # use empl::argument::{Flag, FlagIter};
/// [
///     (
///         "-foo",
///         &[Flag::Short('f'), Flag::Short('o'), Flag::Short('o')] as &[_],
///     ),
///     (
///         "-foo=bar",
///         &[Flag::Short('f'), Flag::Short('o'), Flag::Short('o')],
///     ),
///     ("--help", &[Flag::Long("help")]),
/// ]
/// .into_iter()
/// .for_each(|(l, r)| {
///     assert_eq!(
///         FlagIter::try_from(l)
///             .map(Iterator::collect::<Vec<_>>)
///             .as_deref(),
///         Ok(r)
///     )
/// })
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum FlagIter<'a> {
    Long {
        flag: Option<&'a str>,
        value: Option<&'a str>,
    },
    Short(&'a str),
}
impl<'a> FlagIter<'a> {
    fn t() {
    }

    /// Extract the value from a flag
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::argument::FlagIter;
    /// [
    ///     ("--foo=bar", None, Some("bar")),
    ///     ("-foo=bar", Some(3), Some("bar")),
    /// ]
    /// .into_iter()
    /// .for_each(|(flag, n, val)| {
    ///     let mut flag = FlagIter::try_from(flag).unwrap();
    ///     n.inspect(|n| {
    ///         (0..*n).for_each(|_| {
    ///             let _ = flag.next();
    ///         });
    ///     });
    ///     assert_eq!(flag.value(), val);
    /// })
    /// ```
    pub fn value(self) -> Option<&'a str> {
        match self {
            Self::Long { value, .. } => value,
            Self::Short(short) => Some(short.strip_prefix('=').unwrap_or(short)),
        }
    }
}
impl<'a> Iterator for FlagIter<'a> {
    type Item = Flag<'a>;

    fn next(&mut self) -> Option<Flag<'a>> {
        match self {
            Self::Long { flag, .. } => flag.take().map(Flag::Long),
            Self::Short(tail) if tail.starts_with('=') => None,
            Self::Short(tail) => {
                let mut chars = tail.chars();
                let flag = chars.next()?;
                *tail = chars.as_str();

                Some(Flag::Short(flag))
            }
        }
    }
}
impl<'a> TryFrom<&'a str> for FlagIter<'a> {
    type Error = NonFlagError<'a>;

    fn try_from(flag: &'a str) -> Result<FlagIter<'a>, NonFlagError<'a>> {
        if flag == "--" || flag.len() < 2 {
            Err(NonFlagError(flag))
        } else if let Some(flag) = flag.strip_prefix("--") {
            Ok(flag
                .split_once('=')
                .map(|(flag, value)| FlagIter::Long {
                    flag: Some(flag),
                    value: Some(value),
                })
                .unwrap_or(FlagIter::Long {
                    flag: Some(flag),
                    value: None,
                }))
        } else if let Some(flag) = flag.strip_prefix('-') {
            Ok(FlagIter::Short(flag))
        } else {
            Err(NonFlagError(flag))
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NonFlagError<'a>(pub &'a str);
impl Display for NonFlagError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "`{}` is not a flag", self.0)
    }
}
impl Error for NonFlagError<'_> {}
