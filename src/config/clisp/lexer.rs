use {
    crate::{
        config::clisp::parser::{
            Parser, ParserError, ParserOutput, PureParser,
            token::{Any, Just, Select},
        },
        either::Either,
    },
    unicode_ident::{is_xid_continue, is_xid_start},
};

#[derive(Clone, Copy, Debug)]
pub enum Lexeme<'a> {
    LParen,
    RParen,
    Literal(Literal<'a>),
}

/// [Parser] for [Lexeme]s
#[derive(Clone, Copy, Debug)]
pub struct LexemeParser;
impl<'a> Parser<'a, &'a str> for LexemeParser {
    type Error = ParserError<Either<char, &'a str>>;
    type Output = Lexeme<'a>;

    fn parse(self, input: &'a str) -> Result<ParserOutput<'a, &'a str, Self::Output>, Self::Error> {
        Select((
            Just('(').to(Lexeme::LParen).map_err(|err| err.map(Either::<char, &'a str>::Left)),
            Just(')').to(Lexeme::RParen).map_err(|err| err.map(Either::<char, &'a str>::Left)),
            LiteralParser.map(Lexeme::Literal),
        ))
        .parse(input)
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Literal<'a> {
    Ident(&'a str),
}

/// [Parser] for [Literal]s
#[derive(Clone, Copy, Debug)]
pub struct LiteralParser;
impl<'a> Parser<'a, &'a str> for LiteralParser {
    type Error = ParserError<Either<char, &'a str>>;
    type Output = Literal<'a>;

    fn parse(self, input: &'a str) -> Result<ParserOutput<'a, &'a str, Self::Output>, Self::Error> {
        Any::new()
            .filter(|ch| is_xid_start(*ch), "Xid_Start")
            .then(
                Any::new()
                    .filter(|ch| is_xid_continue(*ch), "Xid_Continue")
                    .repeated(),
            )
            .restore()
            .filter(|ident: &&'a str| !ident.is_empty(), "identifiers cannot be empty")
            .map(Literal::Ident)
            .parse(input)
            .map_err(|err| match err {
                Either::Left(Either::Left(Either::Left(e))) => e.into(),
                Either::Left(Either::Left(Either::Right(e))) => e.map(Either::Left),
                Either::Right(e) => e.map(Either::Right),
            })
    }
}
