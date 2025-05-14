use {
    crate::{
        config::clisp::parser::{
            EofError, Parser, ParserOutput, PureParser,
            token::{Any, Just, Select},
        },
        either::Either,
    },
    std::{
        error::Error,
        fmt::{self, Display, Formatter},
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
            IntParser.map(Literal::Int),
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
            .restore()
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

/// Integer parser
///
/// # Examples
///
/// ```
/// # use empl::config::clisp::{lexer::IntParser, parser::{Parser, ParserOutput}};
/// assert_eq!(IntParser.parse("1000"), Ok(ParserOutput::new("", 1000)));
/// assert_eq!(IntParser.parse("10"), Ok(ParserOutput::new("", 10)));
/// assert_eq!(IntParser.parse("01"), Ok(ParserOutput::new("", 1)));
/// assert_eq!(IntParser.parse("1"), Ok(ParserOutput::new("", 1)));
/// assert_eq!(IntParser.parse("0"), Ok(ParserOutput::new("", 0)));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct IntParser;
impl<'a> Parser<'a, &'a str> for IntParser {
    type Error = IntError;
    type Output = i32;

    fn parse(self, input: &'a str) -> Result<ParserOutput<'a, &'a str, Self::Output>, Self::Error> {
        Any::new()
            .filter_map(|ch: char| {
                ch.to_digit(10)
                    .map(|ch| ch as i32)
                    .ok_or(IntError::NonDigit(ch))
            })
            .map_iter(|iter| {
                let mut iter = iter.peekable();
                iter.peek().ok_or(IntError::Eof(EofError))?;

                iter.try_fold(0_i32, |mut accum, i| {
                    accum = accum.checked_mul(10).ok_or(IntError::Overflow)?;
                    accum = accum.checked_add(i).ok_or(IntError::Overflow)?;

                    Ok(accum)
                })
            })
            .flatten_err()
            .map_err(|Either::Right(err)| err)
            .parse(input)
    }
}
