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
        IdentParser
            .map(Literal::Ident)
            .map_err(LiteralError::Ident)
            .parse(input)
    }
}

#[derive(Clone, Copy, Debug)]
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

#[derive(Clone, Copy, Debug)]
pub struct IdentParser;
impl<'a> Parser<'a, &'a str> for IdentParser {
    type Error = IdentError;
    type Output = &'a str;

    fn parse(self, input: &'a str) -> Result<ParserOutput<'a, &'a str, &'a str>, IdentError> {
        Any::new()
            .filter(|ch| {
                if is_xid_start(*ch) {
                    Ok(())
                } else {
                    Err(IdentError::NotXidStart(*ch))
                }
            })
            .then(
                Any::new()
                    .filter(|ch| {
                        if is_xid_continue(*ch) {
                            Ok(())
                        } else {
                            Err(())
                        }
                    })
                    .repeated(),
            )
            .restore()
            .map_err(|Either::Left(e)| e.into_inner())
            .parse(input)
    }
}
