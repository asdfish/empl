use {
    crate::{
        config::clisp::parser::{Parsable, Parser, ParserError, ParserOutput},
        either::Either,
    },
    std::marker::PhantomData,
};

/// [Parser] created by [Parser::either_or]
#[derive(Clone, Copy, Debug)]
pub struct EitherOr<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I>,
    R: Parser<'a, I>,
{
    pub(super) l: L,
    pub(super) r: R,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, L, R> Parser<'a, I> for EitherOr<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I>,
    R: Parser<'a, I>,
{
    type Error = R::Error;
    type Output = Either<L::Output, R::Output>;

    fn parse(
        self,
        input: I,
    ) -> Result<ParserOutput<'a, I, Self::Output>, ParserError<I::Item, Self::Error>> {
        if let Ok(po) = self.l.parse(input).map(|po| po.map_output(Either::Left)) {
            return Ok(po);
        }

        self.r.parse(input).map(|po| po.map_output(Either::Right))
    }
}

/// [Parser] created by [Parser::map]
#[derive(Clone, Copy, Debug)]
pub struct Map<'a, I, O, P, F>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
    F: FnOnce(P::Output) -> O,
{
    pub(super) parser: P,
    pub(super) map: F,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, O, P, F> Parser<'a, I> for Map<'a, I, O, P, F>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
    F: FnOnce(P::Output) -> O,
{
    type Error = P::Error;
    type Output = O;

    fn parse(
        self,
        input: I,
    ) -> Result<ParserOutput<'a, I, Self::Output>, ParserError<I::Item, Self::Error>> {
        self.parser
            .parse(input)
            .map(move |output| output.map_output(self.map))
    }
}

/// [Parser] created by [Parser::or]
#[derive(Clone, Copy, Debug)]
pub struct Or<'a, I, O, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I, Output = O>,
    R: Parser<'a, I, Output = O>,
{
    pub(super) l: L,
    pub(super) r: R,
    pub(super) _marker: PhantomData<&'a (I, O)>,
}
impl<'a, I, O, L, R> Parser<'a, I> for Or<'a, I, O, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I, Output = O>,
    R: Parser<'a, I, Output = O>,
{
    type Error = R::Error;
    type Output = O;

    fn parse(
        self,
        input: I,
    ) -> Result<ParserOutput<'a, I, Self::Output>, ParserError<I::Item, Self::Error>> {
        if let Ok(output) = self.l.parse(input) {
            Ok(output)
        } else {
            self.r.parse(input)
        }
    }
}

/// [Parser] created by [Parser::then]
#[derive(Clone, Copy, Debug)]
pub struct Then<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I>,
    R: Parser<'a, I>,
{
    pub(super) l: L,
    pub(super) r: R,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, L, R> Parser<'a, I> for Then<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I>,
    R: Parser<'a, I>,
{
    type Error = Either<L::Error, R::Error>;
    type Output = (L::Output, R::Output);

    fn parse(
        self,
        input: I,
    ) -> Result<ParserOutput<'a, I, Self::Output>, ParserError<I::Item, Self::Error>> {
        let items = input.items();
        let ParserOutput {
            next: items,
            output: l,
            ..
        } = self
            .l
            .parse(I::recover(items))
            .map_err(|err| err.map_custom(Either::Left))?;
        let ParserOutput {
            next: items,
            output: r,
            ..
        } = self
            .r
            .parse(items)
            .map_err(|err| err.map_custom(Either::Right))?;

        Ok(ParserOutput::new(items, (l, r)))
    }
}

/// [Parser] created by [Parser::to]
#[derive(Clone, Copy, Debug)]
pub struct To<'a, I, P, T>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
{
    pub(super) parser: P,
    pub(super) to: T,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, P, T> Parser<'a, I> for To<'a, I, P, T>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
{
    type Error = P::Error;
    type Output = T;

    fn parse(
        self,
        input: I,
    ) -> Result<ParserOutput<'a, I, Self::Output>, ParserError<I::Item, Self::Error>> {
        self.parser
            .parse(input)
            .map(move |output| output.map_output(move |_| self.to))
    }
}
