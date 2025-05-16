use {
    crate::config::clisp::parser::{Parsable, Parser, ParserOutput},
    std::{
        cell::Cell,
        marker::{PhantomData, PhantomPinned},
        pin::Pin,
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
/// let expr_parser = expr_parser.declare(|expr| {
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
#[derive(Default)]
#[repr(transparent)]
pub struct RecursiveParser<'a, I, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
{
    parser: Cell<Option<P>>,
    _marker: (PhantomData<&'a I>, PhantomPinned),
}
impl<'a, I, P> RecursiveParser<'a, I, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
{
    /// Create a new recursive parser.
    pub const fn new() -> Self {
        Self {
            parser: Cell::new(None),
            _marker: (PhantomData, PhantomPinned),
        }
    }

    pub fn declare<F>(&'a mut self, declaration: F) -> Pin<&'a Self>
    where
        F: FnOnce(Pin<&'a dyn Parser<'a, I, Error = P::Error, Output = P::Output>>) -> P,
    {
        self.parser
            .set(Some(declaration(unsafe { Pin::new_unchecked(self) })));
        unsafe { Pin::new_unchecked(self) }
    }
}
impl<'a, I, P> Parser<'a, I> for RecursiveParser<'a, I, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
{
    type Error = P::Error;
    type Output = P::Output;

    /// If this is called before [Self::declare] returns, it will panic.
    fn parse(&self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        let parser = Cell::new(None);
        self.parser.swap(&parser);
        let parser = parser
            .into_inner()
            .expect("`RecursiveParser` should not be called before being declared");

        let output = parser.parse(input);
        self.parser.set(Some(parser));

        output
    }
}
