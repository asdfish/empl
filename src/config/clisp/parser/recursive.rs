use {
    crate::config::clisp::parser::{Parsable, Parser, ParserOutput},
    std::{cell::OnceCell, marker::PhantomData},
};

/// Recursive parser creator.
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
///     Some(ParserOutput::new("", Expr::Int(10)))
/// );
/// assert_eq!(
///     expr_parser.parse("-10"),
///     Some(ParserOutput::new("", Expr::Neg(Box::new(Expr::Int(10)))))
/// );
/// ```
#[derive(Clone)]
#[repr(transparent)]
pub struct RecursiveParser<'p, 'src, I, P>
where
    I: Parsable<'src>,
    P: Parser<'src, I>,
{
    parser: OnceCell<P>,
    _marker: PhantomData<&'p &'src I>,
}
impl<'p, 'src, I, P> RecursiveParser<'p, 'src, I, P>
where
    I: Parsable<'src>,
    P: Parser<'src, I>,
{
    pub const fn new() -> Self {
        Self {
            parser: OnceCell::new(),
            _marker: PhantomData,
        }
    }

    /// This function does nothing if the parser was already declared.
    pub fn declare<F>(&'p self, declaration: F)
    where
        F: FnOnce(&'p dyn Parser<'src, I, Output = P::Output>) -> P,
    {
        let result = self.parser.set(declaration(self));
        debug_assert!(result.is_ok());
    }
}
impl<'src, I, P> Default for RecursiveParser<'_, 'src, I, P>
where
    I: Parsable<'src>,
    P: Parser<'src, I>,
{
    fn default() -> Self {
        const { Self::new() }
    }
}
impl<'src, I, P> Parser<'src, I> for RecursiveParser<'_, 'src, I, P>
where
    I: Parsable<'src>,
    P: Parser<'src, I>,
{
    type Output = P::Output;

    /// # Panics
    ///
    /// When debug assertions are enabled, this function will panic if [Self::declare] was not called before this function.
    ///
    /// In release mode, this will simply return [None].
    fn parse(&self, input: I) -> Option<ParserOutput<'src, I, Self::Output>> {
        let parser = self.parser.get();
        debug_assert!(
            parser.is_some(),
            "`RecursiveParser::parse` was called before it was intialized"
        );

        parser.and_then(|parser| parser.parse(input))
    }
}
