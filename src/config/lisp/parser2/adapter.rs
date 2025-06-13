use {
    crate::{
        config::lisp::{
            parser::Parsable,
            parser2::{Parser, ParserInput},
        },
        either::Either,
        ext::pair::{BiFunctor, BiTranspose},
    },
    std::marker::PhantomData,
};

pub struct CoFlatten<'a, I, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
{
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, P> Parser<'a, I> for CoFlatten<'a, I, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
{
    type Output = Option<P::Output>;

    fn parse(
        &self,
        input: ParserInput<'a, I>,
    ) -> Result<(ParserInput<'a, I>, Self::Output), ParserInput<'a, I>> {
        Ok(self
            .parser
            .parse(input)
            .map_or_else(|err| (err, None), |output| output.map_snd(Some)))
    }
}

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
    type Output = Either<L::Output, R::Output>;

    fn parse(
        &self,
        input: ParserInput<'a, I>,
    ) -> Result<(ParserInput<'a, I>, Self::Output), ParserInput<'a, I>> {
        self.l
            .parse(input)
            .map(|output| output.map_snd(Either::Left))
            .or_else(|_| {
                self.r
                    .parse(input)
                    .map(|output| output.map_snd(Either::Right))
            })
    }
}

pub struct Filter<'a, I, P, F>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
    F: Fn(&P::Output) -> bool,
{
    pub(super) parser: P,
    pub(super) predicate: F,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, P, F> Parser<'a, I> for Filter<'a, I, P, F>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
    F: Fn(&P::Output) -> bool,
{
    type Output = P::Output;

    fn parse(
        &self,
        input: ParserInput<'a, I>,
    ) -> Result<(ParserInput<'a, I>, Self::Output), ParserInput<'a, I>> {
        self.parser.parse(input).and_then(|(input, output)| {
            if (self.predicate)(&output) {
                Ok((input, output))
            } else {
                Err(input)
            }
        })
    }
}

pub struct Flatten<'a, I, P, T>
where
    I: Parsable<'a>,
    P: Parser<'a, I, Output = Option<T>>,
{
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, P, T> Parser<'a, I> for Flatten<'a, I, P, T>
where
    I: Parsable<'a>,
    P: Parser<'a, I, Output = Option<T>>,
{
    type Output = T;

    fn parse(
        &self,
        input: ParserInput<'a, I>,
    ) -> Result<(ParserInput<'a, I>, Self::Output), ParserInput<'a, I>> {
        self.parser.parse(input).and_then(|output| {
            output
                .bi_map(Ok::<_, ParserInput<'a, I>>, |output| output.ok_or(input))
                .bi_transpose()
        })
    }
}

pub struct Map<'a, I, P, M, T>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
    M: Fn(P::Output) -> T,
{
    pub(super) parser: P,
    pub(super) morphism: M,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, P, M, T> Parser<'a, I> for Map<'a, I, P, M, T>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
    M: Fn(P::Output) -> T,
{
    type Output = T;

    fn parse(
        &self,
        input: ParserInput<'a, I>,
    ) -> Result<(ParserInput<'a, I>, Self::Output), ParserInput<'a, I>> {
        self.parser
            .parse(input)
            .map(|output| output.map_snd(&self.morphism))
    }
}

pub struct Or<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I>,
    R: Parser<'a, I, Output = L::Output>,
{
    pub(super) l: L,
    pub(super) r: R,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, L, R> Parser<'a, I> for Or<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I>,
    R: Parser<'a, I, Output = L::Output>,
{
    type Output = L::Output;

    fn parse(
        &self,
        input: ParserInput<'a, I>,
    ) -> Result<(ParserInput<'a, I>, Self::Output), ParserInput<'a, I>> {
        self.l.parse(input).or_else(|_| self.r.parse(input))
    }
}

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
    type Output = (L::Output, R::Output);

    fn parse(
        &self,
        input: ParserInput<'a, I>,
    ) -> Result<(ParserInput<'a, I>, Self::Output), ParserInput<'a, I>> {
        self.l
            .parse(input)
            .and_then(|(input, l)| self.r.parse(input).map(move |(input, r)| (input, (l, r))))
    }
}
