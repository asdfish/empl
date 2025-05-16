use {
    crate::{
        config::clisp::parser::{
            EofError, Parser, ParserError, ParserOutput, PureParser,
            token::{Any, Just, Select, Sequence},
        },
        either::Either,
        ext::int::FromStrRadix,
    },
    std::{
        borrow::Cow,
        convert::Infallible,
        error::Error,
        fmt::{self, Display, Formatter},
        marker::PhantomData,
        num::ParseIntError,
    },
    unicode_ident::{is_xid_continue, is_xid_start},
};

#[derive(Clone, Debug)]
pub enum Lexeme<'a> {
    LParen,
    RParen,
    Whitespace,
    Literal(Literal<'a>),
}

#[derive(Clone, Copy, Debug)]
pub struct LexemeParser;
impl<'a> Parser<'a, &'a str> for LexemeParser {
    type Error = LiteralError;
    type Output = Lexeme<'a>;

    fn parse(
        &self,
        input: &'a str,
    ) -> Result<ParserOutput<'a, &'a str, Self::Output>, Self::Error> {
        Select((
            Just('(').map(|_| Lexeme::LParen),
            Just(')').map(|_| Lexeme::RParen),
            WhitespaceParser.map(|_| Lexeme::Whitespace),
            LiteralParser.map(Lexeme::Literal),
        ))
        .parse(input)
    }
}

#[derive(Clone, Debug)]
pub enum Literal<'a> {
    Bool(bool),
    Ident(&'a str),
    Int(i32),
    String(Cow<'a, str>),
    Nil,
}

#[derive(Clone, Debug)]
pub enum LiteralError {
    Ident(IdentError),
    String(StringError),
}
impl Display for LiteralError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Ident(e) => e.fmt(f),
            Self::String(e) => e.fmt(f),
        }
    }
}
impl Error for LiteralError {}
impl From<IdentError> for LiteralError {
    fn from(err: IdentError) -> Self {
        Self::Ident(err)
    }
}
impl From<StringError> for LiteralError {
    fn from(err: StringError) -> Self {
        Self::String(err)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LiteralParser;
impl<'a> Parser<'a, &'a str> for LiteralParser {
    type Error = LiteralError;
    type Output = Literal<'a>;

    fn parse(
        &self,
        input: &'a str,
    ) -> Result<ParserOutput<'a, &'a str, Literal<'a>>, LiteralError> {
        Select((
            Sequence::new("nil").map(|_| Literal::Nil),
            Sequence::new("#t").map(|_| Literal::Bool(true)),
            Sequence::new("#f").map(|_| Literal::Bool(false)),
            IntParser::<10, i32>::new().map(Literal::Int),
            IdentParser.map(Literal::Ident).map_err(LiteralError::Ident),
            StringParser
                .map(Literal::String)
                .map_err(LiteralError::String),
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
/// # use empl::config::clisp::{lexer::{IdentError, IdentParser}, parser::{EofError, Parser, ParserOutput}};
/// assert_eq!(IdentParser.parse(""), Err(IdentError::Eof(EofError)));
/// assert_eq!(IdentParser.parse("foo"), Ok(ParserOutput::new("", "foo")));
/// assert_eq!(IdentParser.parse("1foo"), Err(IdentError::NotXidStart('1')));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct IdentParser;
impl<'a> Parser<'a, &'a str> for IdentParser {
    type Error = IdentError;
    type Output = &'a str;

    fn parse(&self, input: &'a str) -> Result<ParserOutput<'a, &'a str, &'a str>, IdentError> {
        Any::new()
            .map_err(IdentError::Eof)
            .filter(IdentError::NotXidStart, |ch| is_xid_start(*ch))
            .then(
                Any::new()
                    .filter(|_| Default::default(), |ch| is_xid_continue(*ch))
                    .repeated()
                    .map_err(|_: Infallible| unreachable!()),
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
    type Error = ParseIntError;
    type Output = N;

    fn parse(
        &self,
        input: &'a str,
    ) -> Result<ParserOutput<'a, &'a str, Self::Output>, Self::Error> {
        Any::new()
            .filter(|_| Default::default(), |ch: &char| ch.is_digit(RADIX))
            .repeated()
            .map_err(|_: Infallible| unreachable!())
            .map(|digits| N::from_str_radix(digits, RADIX))
            .flatten_err()
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
/// assert_eq!(EscapeCharacterParser.parse("\\u{FACE}"), Ok(ParserOutput::new("", '\u{FACE}')));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct EscapeCharacterParser;
impl<'a> Parser<'a, &'a str> for EscapeCharacterParser {
    type Error = EscapeCharacterError;
    type Output = char;

    fn parse(
        &self,
        input: &'a str,
    ) -> Result<ParserOutput<'a, &'a str, Self::Output>, Self::Error> {
        Just('\\')
            .map_err(EscapeCharacterError::from)
            .ignore_then(
                Just('0')
                    .map_err(EscapeCharacterError::from)
                    .map(|_| '\0')
                    .or(Just('n').map_err(EscapeCharacterError::from).map(|_| '\n'))
                    .or(Just('r').map_err(EscapeCharacterError::from).map(|_| '\r'))
                    .or(Just('t').map_err(EscapeCharacterError::from).map(|_| '\t'))
                    .or(Just('\'').map_err(EscapeCharacterError::from).map(|_| '\''))
                    .or(Just('"').map_err(EscapeCharacterError::from).map(|_| '"'))
                    .or(Just('\\').map_err(EscapeCharacterError::from).map(|_| '\\'))
                    .or(Just('u').map_err(EscapeCharacterError::from).ignore_then(
                        IntParser::<16, u32>::new()
                            .map_err(EscapeCharacterError::ParseUnicode)
                            .map(|i| {
                                char::from_u32(i)
                                    .ok_or(EscapeCharacterError::InvalidUnicodeScalar(i))
                            })
                            .flatten_err()
                            .delimited_by(
                                Just('{').map_err(EscapeCharacterError::from),
                                Just('}').map_err(EscapeCharacterError::from),
                            ),
                    )),
            )
            .parse(input)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum StringError {
    Eof(EofError),
    EscapeCharacter(EscapeCharacterError),
    Unescaped(char),
    Delimiter(char),
}
impl StringError {
    fn delimiter_error(err: ParserError<char>) -> Self {
        match err {
            ParserError::Eof(e) => Self::Eof(e),
            ParserError::Match { found, .. } => Self::Delimiter(found),
        }
    }
}
impl Display for StringError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Eof(e) => e.fmt(f),
            Self::EscapeCharacter(e) => e.fmt(f),
            Self::Unescaped(c) => write!(f, "`{c}` must be escaped"),
            Self::Delimiter(c) => write!(f, "`{c}` was found in place of string delimiter `\"`"),
        }
    }
}
impl Error for StringError {}

/// String parser
///
/// # Examples
///
/// ```
/// # use empl::config::clisp::{lexer::StringParser, parser::{Parser, ParserOutput}};
/// # use std::borrow::Cow;
/// assert_eq!(StringParser.parse(r#""hello world""#), Ok(ParserOutput::new("", Cow::Borrowed("hello world"))));
/// assert_eq!(StringParser.parse(r#""\u{CAFE}""#), Ok(ParserOutput::new("", Cow::Borrowed("\u{CAFE}"))));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct StringParser;
impl<'a> Parser<'a, &'a str> for StringParser {
    type Error = StringError;
    type Output = Cow<'a, str>;

    fn parse(
        &self,
        input: &'a str,
    ) -> Result<ParserOutput<'a, &'a str, Self::Output>, Self::Error> {
        let delimiter = Just('"').map_err(StringError::delimiter_error);

        Any::new()
            .map_err(StringError::Eof)
            .filter(StringError::Unescaped, |ch| '\"'.ne(ch) && '\\'.ne(ch))
            .either_or(EscapeCharacterParser.map_err(StringError::EscapeCharacter))
            .fold(
                || Cow::Borrowed(""),
                |accum, string, ch| match (ch, accum) {
                    (Either::Left(_), Cow::Borrowed(_)) => Ok(Cow::Borrowed(string)),
                    (Either::Left(ch), Cow::Owned(mut string)) => {
                        string.push(ch);
                        Ok(Cow::Owned(string))
                    }
                    (Either::Right(ch), mut string) => {
                        string.to_mut().push(ch);
                        Ok(string)
                    }
                },
            )
            .delimited_by(delimiter, delimiter)
            .parse(input)
    }
}

/// Parser for things like comments, tabs and whitespace
///
/// # Examples
///
/// ```
/// # use empl::config::clisp::{lexer::WhitespaceParser, parser::{Parser, ParserOutput}};
/// assert_eq!(WhitespaceParser.parse("    "), Ok(ParserOutput::new("", ())));
/// assert_eq!(WhitespaceParser.parse("\n\n    ; foo\nbar"), Ok(ParserOutput::new("bar", ())));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct WhitespaceParser;
impl<'a> Parser<'a, &'a str> for WhitespaceParser {
    type Error = EofError;
    type Output = ();

    fn parse(
        &self,
        input: &'a str,
    ) -> Result<ParserOutput<'a, &'a str, Self::Output>, Self::Error> {
        Any::new()
            .map_err(drop)
            .filter(drop, |ch: &char| ch.is_whitespace())
            .map_err(drop)
            .either_or(
                Just(';')
                    .map_err(drop)
                    .then(
                        Any::new()
                            .map_err(drop)
                            .filter(drop, |ch: &char| '\n'.ne(ch))
                            .repeated()
                            .map_err(drop),
                    )
                    .then(Just('\n').map_err(drop)),
            )
            .map_iter(|iter| iter.fold(None, |_, _| Some(())))
            .map_err(|_: Infallible| unreachable!())
            .map(|output| output.ok_or(EofError))
            .flatten_err()
            .parse(input)
    }
}
