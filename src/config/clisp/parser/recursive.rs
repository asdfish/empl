use {
    crate::config::clisp::parser::{Parsable, Parser, ParserOutput},
    std::{
        cell::OnceCell,
        marker::PhantomData,
    },
};

/// Recursive parser creator.
///
/// # Panics
///
/// The parser will panic if you call [RecursiveParser::parser] before [RecursiveParser::declare].
///
/// # Examples
///
/// ```
/// # use empl::config::clisp::{lexer::IntParser, parser::{Parser, ParserOutput, recursive::RecursiveParser, token::Just}};
///
/// #[derive(Debug, PartialEq)]
/// enum Expr {
///     Int(u32),
///     Neg(Box<Self>),
/// }
/// let expr_parser = RecursiveParser::new();
/// expr_parser.declare(|expr| {
///     IntParser::<10, u32>::new()
///         .map(Expr::Int)
///         .or(Just('-').ignore_then(expr.map(Box::new).map(Expr::Neg)))
/// });
/// assert_eq!(
///     expr_parser.parse("10"),
///     Ok(ParserOutput::new("", Expr::Int(10)))
/// );
/// assert_eq!(
///     expr_parser.parse("-10"),
///     Ok(ParserOutput::new("", Expr::Neg(Box::new(Expr::Int(10)))))
/// );
/// ```
#[repr(transparent)]
pub struct RecursiveParser<'a, I, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
{
    parser: OnceCell<P>,
    _marker: PhantomData<&'a I>,
}
impl<'a, I, P> RecursiveParser<'a, I, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
{
    pub const fn new() -> Self {
        Self {
            parser: OnceCell::new(),
            _marker: PhantomData,
        }
    }

    /// This function does nothing if the parser was already declared.
    pub fn declare<F>(&'a self, declaration: F)
    where
        F: FnOnce(&'a dyn Parser<'a, I, Error = P::Error, Output = P::Output>) -> P,
    {
        let result = self.parser.set(declaration(self));
        debug_assert!(result.is_ok());
    }
}
impl<'a, I, P> Parser<'a, I> for RecursiveParser<'a, I, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
{
    type Error = P::Error;
    type Output = P::Output;

    /// # Panics
    ///
    /// If this is called before [Self::declare] returns, it will panic.
    fn parse(&self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        self.parser
            .get()
            .expect("`RecursiveParser` should not be called before being declared")
            .parse(input)
    }
}
