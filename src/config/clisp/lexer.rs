use {
    crate::{
        config::clisp::parser::{
            EofError, Parser, ParserOutput, PureParser,
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

#[derive(Clone, Copy, Debug)]
pub enum Lexeme<'a> {
    LParen,
    RParen,
    Literal(Literal<'a>),
}

#[derive(Clone, Copy, Debug)]
pub struct LexemeParser;
impl<'a> Parser<'a, &'a str> for LexemeParser {
    type Error = LiteralError;
    type Output = Lexeme<'a>;

    fn parse(self, input: &'a str) -> Result<ParserOutput<'a, &'a str, Self::Output>, Self::Error> {
        Select((
            Just('(').to(Lexeme::LParen),
            Just(')').to(Lexeme::RParen),
            LiteralParser.map(Lexeme::Literal),
        ))
        .parse(input)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Literal<'a> {
    Ident(&'a str),
    Int(i32),
}

#[derive(Clone, Copy, Debug)]
pub enum LiteralError {
    Ident(IdentError),
}
impl Display for LiteralError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Ident(e) => e.fmt(f),
        }
    }
}
impl Error for LiteralError {}
impl From<IdentError> for LiteralError {
    fn from(err: IdentError) -> Self {
        Self::Ident(err)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LiteralParser;
impl<'a> Parser<'a, &'a str> for LiteralParser {
    type Error = LiteralError;
    type Output = Literal<'a>;

    fn parse(self, input: &'a str) -> Result<ParserOutput<'a, &'a str, Literal<'a>>, LiteralError> {
        Select((
            IntParser::<10, i32>::new().map(Literal::Int),
            IdentParser.map(Literal::Ident).map_err(LiteralError::Ident),
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

    fn parse(self, input: &'a str) -> Result<ParserOutput<'a, &'a str, &'a str>, IdentError> {
        Any::new()
            .filter(IdentError::NotXidStart, |ch| is_xid_start(*ch))
            .then(
                Any::new()
                    .filter(|_| (), |ch| is_xid_continue(*ch))
                    .repeated(),
            )
            .as_slice()
            .map_err(|Either::Left(e)| e.into_inner())
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
        Self {
            _marker: PhantomData,
        }
    }
}
impl<const RADIX: u32, N> Copy for IntParser<RADIX, N> where N: FromStrRadix {}
impl<'a, const RADIX: u32, N> Parser<'a, &'a str> for IntParser<RADIX, N>
where
    N: FromStrRadix,
{
    type Error = ParseIntError;
    type Output = N;

    fn parse(self, input: &'a str) -> Result<ParserOutput<'a, &'a str, Self::Output>, Self::Error> {
        Any::new()
            .filter(|_| (), |ch: &char| ch.is_digit(RADIX))
            .repeated()
            .map(|digits| N::from_str_radix(digits, RADIX))
            .flatten_err()
            .map_err(|Either::Right(err)| err)
            .parse(input)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StringError<'a> {
    UnknownKeySequence(&'a str),
}

#[derive(Clone, Copy, Debug)]
pub struct StringParser;
impl<'a> Parser<'a, &'a str> for StringParser {
    type Error = StringError<'a>;
    type Output = Cow<'a, str>;

    fn parse(self, _: &'a str) -> Result<ParserOutput<'a, &'a str, Self::Output>, Self::Error> {
        todo!()
    }
}
