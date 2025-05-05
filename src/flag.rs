use std::{
    error::Error,
    fmt::{self, Debug, Display, Formatter},
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
/// # use empl::flag::{Argument, NonFlagError};
/// [
///     ("--", Err(NonFlagError("--"))),
///     ("-", Err(NonFlagError("-"))),
///     ("-a", Ok(Argument::Short("a"))),
///     (
///         "--help",
///         Ok(Argument::Long {
///             flag: Some("help"),
///             value: None,
///         }),
///     ),
///     (
///         "--foo=bar",
///         Ok(Argument::Long {
///             flag: Some("foo"),
///             value: Some("bar"),
///         }),
///     ),
/// ]
/// .into_iter()
/// .for_each(|(l, r)| assert_eq!(Argument::try_from(l), r))
/// ```
///
/// ```
/// # use empl::flag::{Flag, Argument};
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
///         Argument::try_from(l)
///             .map(Iterator::collect::<Vec<_>>)
///             .as_deref(),
///         Ok(r)
///     )
/// })
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum Argument<'a> {
    Long {
        flag: Option<&'a str>,
        value: Option<&'a str>,
    },
    Short(&'a str),
}
impl<'a> Argument<'a> {
    /// Extract the value from a flag
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::flag::Argument;
    /// [
    ///     ("--foo=bar", None, Some("bar")),
    ///     ("-foo=bar", Some(3), Some("bar")),
    /// ]
    /// .into_iter()
    /// .for_each(|(flag, n, val)| {
    ///     let mut flag = Argument::try_from(flag).unwrap();
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
impl<'a> Iterator for Argument<'a> {
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
impl<'a> TryFrom<&'a str> for Argument<'a> {
    type Error = NonFlagError<'a>;

    fn try_from(flag: &'a str) -> Result<Argument<'a>, NonFlagError<'a>> {
        if flag == "--" || flag.len() < 2 {
            Err(NonFlagError(flag))
        } else if let Some(flag) = flag.strip_prefix("--") {
            Ok(flag
                .split_once('=')
                .map(|(flag, value)| Argument::Long {
                    flag: Some(flag),
                    value: Some(value),
                })
                .unwrap_or(Argument::Long {
                    flag: Some(flag),
                    value: None,
                }))
        } else if let Some(flag) = flag.strip_prefix('-') {
            Ok(Argument::Short(flag))
        } else {
            Err(NonFlagError(flag))
        }
    }
}

/// [Iterator] for [Flag]s in argv.
///
/// # Examples
///
/// ```
/// # use empl::flag::{Arguments, Flag};
/// # use std::convert::Infallible;
/// [
///     (
///         &["--help", "-lsh"] as &[_],
///         &[
///             Flag::Long("help"),
///             Flag::Short('l'),
///             Flag::Short('s'),
///             Flag::Short('h'),
///         ] as &[_],
///     ),
///     (
///         &["--help", "-lsh", "--", "--foo", "hello", "world"] as &[_],
///         &[
///             Flag::Long("help"),
///             Flag::Short('l'),
///             Flag::Short('s'),
///             Flag::Short('h'),
///         ] as &[_],
///     ),
///     (
///         &["-foo", "--bar"],
///         &[
///             Flag::Short('f'),
///             Flag::Short('o'),
///             Flag::Short('o'),
///             Flag::Long("bar"),
///         ],
///     ),
/// ]
/// .into_iter()
/// .for_each(|(l, r)| {
///     assert_eq!(
///         Arguments::new(l.iter().copied().map(Ok::<_, Infallible>))
///             .collect::<Result<Vec<_>, _>>()
///             .as_deref(),
///         Ok(r)
///     )
/// })
/// ```
#[derive(Clone, Debug)]
pub struct Arguments<'a, I, E>
where
    I: Iterator<Item = Result<&'a str, E>>,
{
    arg: Option<Argument<'a>>,
    passed_separator: bool,
    src: I,
}
impl<'a, I, E> Arguments<'a, I, E>
where
    I: Iterator<Item = Result<&'a str, E>>,
{
    pub const fn new(src: I) -> Self {
        Self {
            arg: None,
            passed_separator: false,
            src,
        }
    }

    /// # Examples
    ///
    /// ```
    /// # use empl::flag::Arguments;
    /// # use std::convert::Infallible;
    /// [
    ///     (&["--foo", "bar"] as &[_], 1, Ok("bar")),
    ///     (&["--foo=baz"], 1, Ok("baz")),
    ///     (&["foo"], 0, Ok("foo"))
    /// ]
    /// .into_iter()
    /// .for_each(|(i, s, o)| {
    ///     let mut iter = Arguments::new(i.iter().copied().map(Ok::<_, Infallible>));
    ///     (0..s).for_each(|_| {
    ///         let _ = iter.next();
    ///     });
    ///     assert_eq!(iter.value(), Some(o));
    /// })
    /// ```
    pub fn value(&mut self) -> Option<Result<&'a str, E>> {
        self.arg
            .take()
            .and_then(|arg| arg.value().map(Ok).or_else(|| self.src.next()))
            .or_else(|| self.src.next())
            .filter(|arg| arg.as_ref().map(|arg| !arg.starts_with('-')).unwrap_or(true))
    }
}

impl<'a, I, E> Iterator for Arguments<'a, I, E>
where
    I: Iterator<Item = Result<&'a str, E>>,
{
    type Item = Result<Flag<'a>, ArgumentsError<'a, E>>;

    fn next(&mut self) -> Option<Result<Flag<'a>, ArgumentsError<'a, E>>> {
        match &mut self.arg {
            _ if self.passed_separator => None,
            Some(arg) => match arg.next() {
                Some(arg) => Some(Ok(arg)),
                None => {
                    self.arg = None;
                    self.next()
                }
            },
            None => {
                match self
                    .src
                    .next()
                    .filter(|arg| {
                        if matches!(arg, Ok("--")) {
                            self.passed_separator = true;
                            false
                        } else {
                            true
                        }
                    })?
                    .map_err(ArgumentsError::Source)
                    .and_then(|arg| Argument::try_from(arg).map_err(ArgumentsError::NonFlag))
                {
                    Ok(arg) => {
                        self.arg = Some(arg);
                        self.next()
                    }
                    Err(err) => Some(Err(err)),
                }
            }
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum ArgumentsError<'a, E> {
    NonFlag(NonFlagError<'a>),
    Source(E),
}
impl<E> Display for ArgumentsError<'_, E>
where
    E: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::NonFlag(e) => Display::fmt(e, f),
            Self::Source(e) => write!(f, "failed to source argument: {}", e),
        }
    }
}
impl<E> Error for ArgumentsError<'_, E> where E: Debug + Display {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NonFlagError<'a>(pub &'a str);
impl Display for NonFlagError<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "`{}` is not a flag", self.0)
    }
}
impl Error for NonFlagError<'_> {}
