use {
    crate::config::clisp::{
        evaluator::Value,
        lexer::{Lexeme, Literal},
        parser::{
            Parser, ParserOutput,
            recursive::RecursiveParser,
            token::{Any, Just},
        },
    },
    std::collections::VecDeque,
};

#[derive(Clone, Debug, PartialEq)]
pub enum Expr<'a> {
    List(VecDeque<Self>),
    Literal(&'a Literal<'a>),
    /// Should not be used when parsing tokens. This is just for calling [ClispFn][crate::config::clisp::evaluator::ClispFn] with pre-existing values.
    Value(Value<'a>),
}

#[derive(Clone, Copy, Debug)]
pub struct ExprParser;
impl<'a> Parser<'a, &'a [Lexeme<'a>]> for ExprParser {
    type Output = Expr<'a>;

    fn parse(
        &self,
        input: &'a [Lexeme<'a>],
    ) -> Option<ParserOutput<'a, &'a [Lexeme<'a>], Self::Output>> {
        let parser = RecursiveParser::new();
        parser.declare(|expr| {
            Just(&Lexeme::Whitespace)
                .maybe()
                .ignore_then(
                    expr.then(
                        Just(&Lexeme::Whitespace)
                            .ignore_then(expr)
                            .map_iter(|iter| iter.collect::<VecDeque<_>>()),
                    )
                    .maybe()
                    .delimited_by(
                        Just(&Lexeme::LParen).then(Just(&Lexeme::Whitespace).maybe()),
                        Just(&Lexeme::Whitespace)
                            .maybe()
                            .then(Just(&Lexeme::RParen)),
                    )
                    .map(|args| {
                        args.map(|(head, mut tail)| {
                            tail.push_front(head);
                            tail
                        })
                        .unwrap_or_default()
                    })
                    .map(Expr::List)
                    .or(Any::new()
                        .filter_map(|lexeme: &'a Lexeme<'a>| match lexeme {
                            Lexeme::Literal(literal) => Some(literal),
                            _ => None,
                        })
                        .map(Expr::Literal)),
                )
                .then_ignore(Just(&Lexeme::Whitespace).maybe())
        });

        parser.parse(input)
    }
}
