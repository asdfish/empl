use {
    crate::{
        config::clisp::parser::{
            EofError, Parser, ParserError, ParserOutput, PureParser,
            token::{Any, Just, Select},
        },
        either::Either,
        ext::int::FromStrRadix,
    },
    std::{
        borrow::Cow,
        error::Error,
        fmt::{self, Display, Formatter},
        marker::PhantomData,
        num::ParseIntError,
    },
    unicode_ident::{is_xid_continue, is_xid_start},
};

#[derive(Clone, Debug, PartialEq)]
pub enum Lexeme<'a> {
    LParen,
    RParen,
    Whitespace,
    Literal(Literal<'a>),
}

#[derive(Clone, Copy, Debug)]
pub struct LexemeParser;
impl<'a> Parser<'a, &'a str> for LexemeParser {
    type Output = Lexeme<'a>;

    fn parse(&self, input: &'a str) -> Option<ParserOutput<'a, &'a str, Self::Output>> {
        Select((
            Just('(').map(|_| Lexeme::LParen),
            Just(')').map(|_| Lexeme::RParen),
            WhitespaceParser.map(|_| Lexeme::Whitespace),
            LiteralParser.map(Lexeme::Literal),
        ))
        .parse(input)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Literal<'a> {
    Bool(bool),
    Ident(&'a str),
    Int(i32),
    String(Cow<'a, str>),
}

#[derive(Clone, Copy, Debug)]
pub struct LiteralParser;
impl<'a> Parser<'a, &'a str> for LiteralParser {
    type Output = Literal<'a>;

    fn parse(&self, input: &'a str) -> Option<ParserOutput<'a, &'a str, Literal<'a>>> {
        Select((
            Just('#').ignore_then(
                Just('t')
                    .map(|_| Literal::Bool(true))
                    .or(Just('f').map(|_| Literal::Bool(false))),
            ),
            IntParser::<10, i32>::new().map(Literal::Int),
            IdentParser.map(Literal::Ident),
            StringParser.map(Literal::String),
        ))
        .parse(input)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IdentError {
    Eof(EofError),
    NotXidStart(char),
}
impl Display for IdentError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Eof(e) => e.fmt(f),
            Self::NotXidStart(ch) => write!(f, "`{ch}` is not `Xid_Start`"),
        }
    }
}
impl Error for IdentError {}
impl From<EofError> for IdentError {
    fn from(err: EofError) -> Self {
        Self::Eof(err)
    }
}

/// Identifier parser.
///
/// # Examples
///
/// ```
/// # use empl::config::clisp::{lexer::IdentParser, parser::{Parser, ParserOutput}};
/// assert_eq!(IdentParser.parse(""), None);
/// assert_eq!(IdentParser.parse("foo"), Some(ParserOutput::new("", "foo")));
/// assert_eq!(IdentParser.parse("1foo"), None);
/// ```
#[derive(Clone, Copy, Debug)]
pub struct IdentParser;
impl<'a> Parser<'a, &'a str> for IdentParser {
    type Output = &'a str;

    fn parse(&self, input: &'a str) -> Option<ParserOutput<'a, &'a str, &'a str>> {
        Any::new()
            .filter(|ch| is_xid_start(*ch))
            .then(
                Any::new()
                    .filter(|ch: &char| is_xid_continue(*ch))
                    .repeated(),
            )
            .as_slice()
            .parse(input)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum IntError {
    Eof(EofError),
    NonDigit(char),
    Overflow,
}
impl Display for IntError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Eof(e) => e.fmt(f),
            Self::NonDigit(ch) => write!(f, "`{ch}` is not a digit"),
            Self::Overflow => f.write_str("integer is too large"),
        }
    }
}
impl Error for IntError {}
impl From<EofError> for IntError {
    fn from(err: EofError) -> Self {
        Self::Eof(err)
    }
}

#[derive(Debug, Default)]
pub struct IntParser<const RADIX: u32, N>
where
    N: FromStrRadix,
{
    _marker: PhantomData<N>,
}
impl<const RADIX: u32, N> IntParser<RADIX, N>
where
    N: FromStrRadix,
{
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}
impl<const RADIX: u32, N> Clone for IntParser<RADIX, N>
where
    N: FromStrRadix,
{
    fn clone(&self) -> Self {
        *self
    }
}
impl<const RADIX: u32, N> Copy for IntParser<RADIX, N> where N: FromStrRadix {}
impl<'a, const RADIX: u32, N> Parser<'a, &'a str> for IntParser<RADIX, N>
where
    N: FromStrRadix,
{
    type Output = N;

    fn parse(&self, input: &'a str) -> Option<ParserOutput<'a, &'a str, Self::Output>> {
        Any::new()
            .filter(|ch: &char| ch.is_digit(RADIX))
            .repeated()
            .map(|digits| N::from_str_radix(digits, RADIX).ok())
            .flatten()
            .parse(input)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum EscapeCharacterError {
    Eof(EofError),
    InvalidUnicodeScalar(u32),
    ParseUnicode(ParseIntError),
    UnknownEscape(char),
}
impl Display for EscapeCharacterError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Eof(e) => e.fmt(f),
            Self::InvalidUnicodeScalar(i) => write!(f, "`{i:x}` is not a valid unicode scalar"),
            Self::ParseUnicode(e) => write!(f, "failed to parse unicode scalar value: {e}"),
            Self::UnknownEscape(ch) => write!(f, "unknown escape character `{ch}`"),
        }
    }
}
impl Error for EscapeCharacterError {}
impl From<ParseIntError> for EscapeCharacterError {
    fn from(err: ParseIntError) -> Self {
        Self::ParseUnicode(err)
    }
}
impl From<ParserError<char>> for EscapeCharacterError {
    fn from(err: ParserError<char>) -> Self {
        match err {
            ParserError::Eof(e) => Self::Eof(e),
            ParserError::Match { found, .. } => Self::UnknownEscape(found),
        }
    }
}

/// Escape code parser
///
/// # Examples
///
/// ```
/// # use empl::config::clisp::{parser::{Parser, ParserOutput}, lexer::EscapeCharacterParser};
/// assert_eq!(EscapeCharacterParser.parse("\\u{FACE}"), Some(ParserOutput::new("", '\u{FACE}')));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct EscapeCharacterParser;
impl<'a> Parser<'a, &'a str> for EscapeCharacterParser {
    type Output = char;

    fn parse(&self, input: &'a str) -> Option<ParserOutput<'a, &'a str, Self::Output>> {
        Just('\\')
            .ignore_then(
                Just('0')
                    .map(|_| '\0')
                    .or(Just('n').map(|_| '\n'))
                    .or(Just('r').map(|_| '\r'))
                    .or(Just('t').map(|_| '\t'))
                    .or(Just('\'').map(|_| '\''))
                    .or(Just('"').map(|_| '"'))
                    .or(Just('\\').map(|_| '\\'))
                    .or(Just('u').ignore_then(
                        IntParser::<16, u32>::new()
                            .map(char::from_u32)
                            .flatten()
                            .delimited_by(Just('{'), Just('}')),
                    )),
            )
            .parse(input)
    }
}

/// String parser
///
/// # Examples
///
/// ```
/// # use empl::config::clisp::{lexer::StringParser, parser::{Parser, ParserOutput}};
/// # use std::borrow::Cow;
/// assert_eq!(StringParser.parse(r#""hello world""#), Some(ParserOutput::new("", Cow::Borrowed("hello world"))));
/// assert_eq!(StringParser.parse(r#""\u{CAFE}""#), Some(ParserOutput::new("", Cow::Borrowed("\u{CAFE}"))));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct StringParser;
impl<'a> Parser<'a, &'a str> for StringParser {
    type Output = Cow<'a, str>;

    fn parse(&self, input: &'a str) -> Option<ParserOutput<'a, &'a str, Self::Output>> {
        Any::new()
            .filter(|ch: &char| '\"'.ne(ch) && '\\'.ne(ch))
            .either_or(EscapeCharacterParser)
            .fold(
                || Cow::Borrowed(""),
                |accum, string, ch| match (ch, accum) {
                    (Either::Left(_), Cow::Borrowed(_)) => Some(Cow::Borrowed(string)),
                    (Either::Left(ch), Cow::Owned(mut string)) => {
                        string.push(ch);
                        Some(Cow::Owned(string))
                    }
                    (Either::Right(ch), mut string) => {
                        string.to_mut().push(ch);
                        Some(string)
                    }
                },
            )
            .delimited_by(Just('"'), Just('"'))
            .parse(input)
    }
}

/// Parser for things like comments, tabs and whitespace
///
/// # Examples
///
/// ```
/// # use empl::config::clisp::{lexer::WhitespaceParser, parser::{Parser, ParserOutput}};
/// assert_eq!(WhitespaceParser.parse("    "), Some(ParserOutput::new("", ())));
/// assert_eq!(WhitespaceParser.parse("\n\n    ; foo\nbar"), Some(ParserOutput::new("bar", ())));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct WhitespaceParser;
impl<'a> Parser<'a, &'a str> for WhitespaceParser {
    type Output = ();

    fn parse(&self, input: &'a str) -> Option<ParserOutput<'a, &'a str, Self::Output>> {
        Any::new()
            .filter(|ch: &char| ch.is_whitespace())
            .either_or(
                Just(';')
                    .then(Any::new().filter(|ch: &char| '\n'.ne(ch)).repeated())
                    .then(Just('\n')),
            )
            .map_iter(|iter| iter.fold(None, |_, _| Some(())))
            .flatten()
            .parse(input)
    }
}
