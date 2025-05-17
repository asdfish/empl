use {
    crate::config::clisp::{
        lexer::{Lexeme, Literal},
        parser::{
            EofError, Parser, ParserError, ParserOutput,
            recursive::RecursiveParser,
            token::{Any, Just},
        },
    },
    nonempty_collections::NEVec,
    std::convert::Infallible,
};

#[derive(Clone, Debug)]
pub enum Expr<'a> {
    Apply(NEVec<Self>),
    Literal(&'a Literal<'a>),
}

pub enum ExprError<'a> {
    Eof(EofError),
    Delimiter(ParserError<&'a Lexeme<'a>>),
    Whitespace(ParserError<&'a Lexeme<'a>>),
    NonLiteral(&'a Lexeme<'a>),
}

#[derive(Clone, Copy, Debug)]
pub struct ExprParser;
impl<'a> Parser<'a, &'a [Lexeme<'a>]> for ExprParser {
    type Error = ExprError<'a>;
    type Output = Expr<'a>;

    fn parse(
        &self,
        input: &'a [Lexeme<'a>],
    ) -> Result<ParserOutput<'a, &'a [Lexeme<'a>], Self::Output>, Self::Error> {
        let parser = RecursiveParser::new();
        parser.declare(|expr| {
            expr.then(
                Just(&Lexeme::Whitespace)
                    .map_err(ExprError::Whitespace)
                    .ignore_then(expr)
                    .map_iter(|iter| iter.collect::<Vec<_>>())
                    .map_err(|_: Infallible| unreachable!()),
            )
            .delimited_by(
                Just(&Lexeme::LParen)
                    .then(
                        Just(&Lexeme::Whitespace)
                            .maybe()
                            .map_err(|_: Infallible| unreachable!()),
                    )
                    .map_err(ExprError::Delimiter),
                Just(&Lexeme::Whitespace)
                    .maybe()
                    .map_err(|_: Infallible| unreachable!())
                    .then(Just(&Lexeme::RParen))
                    .map_err(ExprError::Delimiter),
            )
            .map(NEVec::from)
            .map(Expr::Apply)
            .or(Any::new()
                .map_err(ExprError::Eof)
                .filter_map(|lexeme: &'a Lexeme<'a>| match lexeme {
                    Lexeme::Literal(literal) => Ok(literal),
                    lexeme => Err(ExprError::NonLiteral(lexeme)),
                })
                .map(Expr::Literal))
        });

        parser.parse(input)
    }
}
