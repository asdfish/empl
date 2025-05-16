//! Parser adapters
//!
//! Having a billion type parameters for these types will improves error messages.
//!
//! Some parser adapters manually implement [Clone] and [Copy] because the automatically derived implementations require all type parameters to implement [Clone] and [Copy].

use {
    crate::{
        config::clisp::parser::{Parsable, Parser, ParserOutput, PureParser},
        either::Either,
    },
    std::{convert::Infallible, marker::PhantomData},
};

/// [Parser] created by [PureParser::as_slice]
#[derive(Clone, Copy, Debug)]
pub struct AsSlice<'a, I, P>
where
    I: Parsable<'a>,
    P: PureParser<'a, I> + Sized,
{
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, P> Parser<'a, I> for AsSlice<'a, I, P>
where
    I: Parsable<'a>,
    P: PureParser<'a, I> + Sized,
{
    type Error = P::Error;
    type Output = I;

    fn parse(&self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        self.parser
            .parse(input)
            .map(|ParserOutput { output, .. }| output)
            .map(P::output_len)
            .map(|split| input.split_at(split))
            .map(|(output, next)| ParserOutput::new(next, output))
    }
}
unsafe impl<'a, I, P> PureParser<'a, I> for AsSlice<'a, I, P>
where
    I: Parsable<'a>,
    P: PureParser<'a, I> + Sized,
{
    fn output_len(output: Self::Output) -> usize {
        I::items_len(output)
    }
}

/// [Parser] created by [Parser::co_flatten_err]
#[derive(Clone, Copy, Debug)]
pub struct CoFlattenErr<'a, I, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I> + Sized,
{
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, P> Parser<'a, I> for CoFlattenErr<'a, I, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I> + Sized,
{
    type Error = Infallible;
    type Output = Result<P::Output, P::Error>;

    fn parse(&self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        Ok(self
            .parser
            .parse(input)
            .map(|output| output.map_output(Ok))
            .map_err(Err)
            .unwrap_or_else(|err| ParserOutput::new(input, err)))
    }
}

/// [Parser] created by [Parser::delimited_by]
#[derive(Debug)]
pub struct DelimitedBy<'a, I, E, L, P, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I, Error = E>,
    P: Parser<'a, I, Error = E>,
    R: Parser<'a, I, Error = E>,
{
    pub(super) l: L,
    pub(super) parser: P,
    pub(super) r: R,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, E, L, P, R> Clone for DelimitedBy<'a, I, E, L, P, R>
where
    I: Parsable<'a>,
    L: Clone + Parser<'a, I, Error = E>,
    P: Clone + Parser<'a, I, Error = E>,
    R: Clone + Parser<'a, I, Error = E>,
{
    fn clone(&self) -> Self {
        Self {
            l: self.l.clone(),
            parser: self.parser.clone(),
            r: self.r.clone(),
            _marker: PhantomData,
        }
    }
}
impl<'a, I, E, L, P, R> Copy for DelimitedBy<'a, I, E, L, P, R>
where
    I: Parsable<'a>,
    L: Clone + Copy + Parser<'a, I, Error = E>,
    P: Clone + Copy + Parser<'a, I, Error = E>,
    R: Clone + Copy + Parser<'a, I, Error = E>,
{
}
impl<'a, I, E, L, P, R> Parser<'a, I> for DelimitedBy<'a, I, E, L, P, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I, Error = E>,
    P: Parser<'a, I, Error = E>,
    R: Parser<'a, I, Error = E>,
{
    type Error = E;
    type Output = P::Output;

    fn parse(&self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        let ParserOutput { next: input, .. } = self.l.parse(input)?;
        let ParserOutput {
            next: input,
            output,
            ..
        } = self.parser.parse(input)?;
        let ParserOutput { next: input, .. } = self.r.parse(input)?;

        Ok(ParserOutput::new(input, output))
    }
}

/// [Parser] created by [Parser::either_or]
#[derive(Clone, Copy, Debug)]
pub struct EitherOr<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I> + Sized,
    R: Parser<'a, I> + Sized,
{
    pub(super) l: L,
    pub(super) r: R,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, L, R> Parser<'a, I> for EitherOr<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I> + Sized,
    R: Parser<'a, I> + Sized,
{
    type Error = R::Error;
    type Output = Either<L::Output, R::Output>;

    fn parse(&self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        if let Ok(po) = self.l.parse(input).map(|po| po.map_output(Either::Left)) {
            return Ok(po);
        }

        self.r.parse(input).map(|po| po.map_output(Either::Right))
    }
}
// SAFETY: both parsers must be pure
unsafe impl<'a, I, L, R> PureParser<'a, I> for EitherOr<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I> + PureParser<'a, I> + Sized,
    R: Parser<'a, I> + PureParser<'a, I> + Sized,
{
    fn output_len(output: Self::Output) -> usize {
        match output {
            Either::Left(output) => L::output_len(output),
            Either::Right(output) => R::output_len(output),
        }
    }
}

/// [Parser] created by [Parser::filter]
#[derive(Debug)]
pub struct Filter<'a, E, F, I, P>
where
    I: Parsable<'a>,
    E: Fn(P::Output) -> P::Error,
    F: Fn(&P::Output) -> bool,
    P: Parser<'a, I> + Sized,
{
    pub(super) error: E,
    pub(super) parser: P,
    pub(super) predicate: F,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, E, F, I, P> Clone for Filter<'a, E, F, I, P>
where
    I: Parsable<'a>,
    E: Clone + Fn(P::Output) -> P::Error,
    F: Clone + Fn(&P::Output) -> bool,
    P: Clone + Parser<'a, I> + Sized,
{
    fn clone(&self) -> Self {
        Self {
            error: self.error.clone(),
            parser: self.parser.clone(),
            predicate: self.predicate.clone(),
            _marker: PhantomData,
        }
    }
}
impl<'a, E, F, I, P> Copy for Filter<'a, E, F, I, P>
where
    I: Parsable<'a>,
    E: Clone + Copy + Fn(P::Output) -> P::Error,
    F: Clone + Copy + Fn(&P::Output) -> bool,
    P: Clone + Copy + Parser<'a, I> + Sized,
{
}
impl<'a, E, F, I, P> Parser<'a, I> for Filter<'a, E, F, I, P>
where
    I: Parsable<'a>,
    E: Fn(P::Output) -> P::Error,
    F: Fn(&P::Output) -> bool,
    P: Parser<'a, I> + Sized,
{
    type Error = P::Error;
    type Output = P::Output;

    fn parse(&self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        let output = self.parser.parse(input)?;

        if (self.predicate)(&output.output) {
            Ok(output)
        } else {
            Err((self.error)(output.output))
        }
    }
}
unsafe impl<'a, E, F, I, P> PureParser<'a, I> for Filter<'a, E, F, I, P>
where
    I: Parsable<'a>,
    E: Fn(P::Output) -> P::Error,
    F: Fn(&P::Output) -> bool,
    P: Parser<'a, I> + PureParser<'a, I> + Sized,
{
    fn output_len(output: Self::Output) -> usize {
        P::output_len(output)
    }
}

/// [Parser] created by [Parser::filter_map]
#[derive(Debug)]
pub struct FilterMap<'a, E, I, M, P, T>
where
    I: Parsable<'a>,
    M: Fn(P::Output) -> Result<T, E>,
    P: Parser<'a, I, Error = E>,
{
    pub(super) map: M,
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, E, I, M, P, T> Clone for FilterMap<'a, E, I, M, P, T>
where
    I: Parsable<'a>,
    M: Clone + Fn(P::Output) -> Result<T, E>,
    P: Clone + Parser<'a, I, Error = E>,
{
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
            parser: self.parser.clone(),
            _marker: PhantomData,
        }
    }
}
impl<'a, E, I, M, P, T> Copy for FilterMap<'a, E, I, M, P, T>
where
    I: Parsable<'a>,
    M: Clone + Copy + Fn(P::Output) -> Result<T, E>,
    P: Clone + Copy + Parser<'a, I, Error = E>,
{
}
impl<'a, E, I, M, P, T> Parser<'a, I> for FilterMap<'a, E, I, M, P, T>
where
    I: Parsable<'a>,
    M: Fn(P::Output) -> Result<T, E>,
    P: Parser<'a, I, Error = E>,
{
    type Error = E;
    type Output = T;

    fn parse(&self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        self.parser.parse(input)?.map_output(&self.map).transpose()
    }
}

/// [Parser] created by [Parser::flatten_err]
#[derive(Debug)]
pub struct FlattenErr<'a, I, E, O, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I, Error = E, Output = Result<O, E>>,
{
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, E, O, P> Clone for FlattenErr<'a, I, E, O, P>
where
    I: Parsable<'a>,
    P: Clone + Parser<'a, I, Error = E, Output = Result<O, E>>,
{
    fn clone(&self) -> Self {
        Self {
            parser: self.parser.clone(),
            _marker: PhantomData,
        }
    }
}
impl<'a, I, E, O, P> Copy for FlattenErr<'a, I, E, O, P>
where
    I: Parsable<'a>,
    P: Clone + Copy + Parser<'a, I, Error = E, Output = Result<O, E>>,
{
}
impl<'a, I, E, O, P> Parser<'a, I> for FlattenErr<'a, I, E, O, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I, Error = E, Output = Result<O, E>>,
{
    type Error = E;
    type Output = O;

    fn parse(&self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        self.parser
            .parse(input)
            .and_then(|output| output.transpose())
    }
}

/// [Parser] created by [Parser::fold]
#[derive(Debug)]
pub struct Fold<'a, A, AF, E, F, I, P>
where
    AF: Fn() -> A,
    I: Parsable<'a>,
    F: Fn(A, I, P::Output) -> Result<A, E>,
    P: Clone + Parser<'a, I> + Sized,
{
    pub(super) fold: F,
    pub(super) parser: P,
    pub(super) start: AF,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, A, AF, E, F, I, P> Clone for Fold<'a, A, AF, E, F, I, P>
where
    I: Parsable<'a>,
    AF: Clone + Fn() -> A,
    F: Clone + Fn(A, I, P::Output) -> Result<A, E>,
    P: Clone + Parser<'a, I> + Sized,
{
    fn clone(&self) -> Self {
        Self {
            fold: self.fold.clone(),
            parser: self.parser.clone(),
            start: self.start.clone(),
            _marker: PhantomData,
        }
    }
}
impl<'a, A, AF, E, F, I, P> Copy for Fold<'a, A, AF, E, F, I, P>
where
    I: Parsable<'a>,
    AF: Clone + Copy + Fn() -> A,
    F: Clone + Copy + Fn(A, I, P::Output) -> Result<A, E>,
    P: Clone + Copy + Parser<'a, I> + Sized,
{
}
impl<'a, A, AF, E, F, I, P> Parser<'a, I> for Fold<'a, A, AF, E, F, I, P>
where
    AF: Fn() -> A,
    I: Parsable<'a>,
    F: Fn(A, I, P::Output) -> Result<A, E>,
    P: Clone + Parser<'a, I> + Sized,
{
    type Error = E;
    type Output = A;

    fn parse(&self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        let mut accum = (self.start)();
        let mut items = input;

        while let Ok(ParserOutput { next, output, .. }) = self.parser.clone().parse(items) {
            let Some((slice, _)) = input.split_at_checked(input.items_len() - next.items_len())
            else {
                break;
            };
            accum = (self.fold)(accum, slice, output)?;

            items = next;
        }

        Ok(ParserOutput::new(items, accum))
    }
}

/// [Parser] created by [Parser::ignore_then]
#[derive(Debug)]
pub struct IgnoreThen<'a, I, E, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I, Error = E>,
    R: Parser<'a, I, Error = E>,
{
    pub(super) l: L,
    pub(super) r: R,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, E, L, R> Clone for IgnoreThen<'a, I, E, L, R>
where
    I: Parsable<'a>,
    L: Clone + Parser<'a, I, Error = E>,
    R: Clone + Parser<'a, I, Error = E>,
{
    fn clone(&self) -> Self {
        Self {
            l: self.l.clone(),
            r: self.r.clone(),
            _marker: PhantomData,
        }
    }
}
impl<'a, I, E, L, R> Copy for IgnoreThen<'a, I, E, L, R>
where
    I: Parsable<'a>,
    L: Clone + Copy + Parser<'a, I, Error = E>,
    R: Clone + Copy + Parser<'a, I, Error = E>,
{
}
impl<'a, I, E, L, R> Parser<'a, I> for IgnoreThen<'a, I, E, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I, Error = E>,
    R: Parser<'a, I, Error = E>,
{
    type Error = E;
    type Output = R::Output;

    fn parse(&self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        let ParserOutput { next: input, .. } = self.l.parse(input)?;
        let ParserOutput {
            next: input,
            output,
            ..
        } = self.r.parse(input)?;

        Ok(ParserOutput::new(input, output))
    }
}

/// [Iterator] for [Parser]s
#[derive(Clone, Copy, Debug)]
pub struct Iter<'a, I, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I> + Sized,
{
    input: I,
    parser: P,
    _marker: PhantomData<&'a ()>,
}
impl<'a, I, P> Iterator for Iter<'a, I, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I> + Sized,
{
    type Item = P::Output;

    fn next(&mut self) -> Option<P::Output> {
        self.parser
            .parse(self.input)
            .map(|ParserOutput { next, output, .. }| {
                self.input = next;
                output
            })
            .ok()
    }
}

/// [Parser] created by [Parser::map]
#[derive(Debug)]
pub struct Map<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: Fn(P::Output) -> O,
    P: Parser<'a, I> + Sized,
{
    pub(super) map: F,
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, O, F, P> Clone for Map<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: Clone + Fn(P::Output) -> O,
    P: Clone + Parser<'a, I> + Sized,
{
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
            parser: self.parser.clone(),
            _marker: PhantomData,
        }
    }
}
impl<'a, I, O, F, P> Copy for Map<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: Clone + Copy + Fn(P::Output) -> O,
    P: Clone + Copy + Parser<'a, I> + Sized,
{
}
impl<'a, I, O, F, P> Parser<'a, I> for Map<'a, I, F, O, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I> + Sized,
    F: Fn(P::Output) -> O,
{
    type Error = P::Error;
    type Output = O;

    fn parse(&self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        self.parser
            .parse(input)
            .map(move |output| output.map_output(&self.map))
    }
}

/// [Parser] created by [Parser::map_err]
#[derive(Debug)]
pub struct MapErr<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: Fn(P::Error) -> O,
    P: Parser<'a, I> + Sized,
{
    pub(super) map: F,
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, F, O, P> Clone for MapErr<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: Clone + Fn(P::Error) -> O,
    P: Clone + Parser<'a, I> + Sized,
{
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
            parser: self.parser.clone(),
            _marker: PhantomData,
        }
    }
}
impl<'a, I, F, O, P> Copy for MapErr<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: Clone + Copy + Fn(P::Error) -> O,
    P: Clone + Copy + Parser<'a, I> + Sized,
{
}
impl<'a, I, F, O, P> Parser<'a, I> for MapErr<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: Fn(P::Error) -> O,
    P: Parser<'a, I> + Sized,
{
    type Error = O;
    type Output = P::Output;

    fn parse(&self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        self.parser.parse(input).map_err(&self.map)
    }
}
// SAFETY: should be safe if `P` is pure
unsafe impl<'a, I, F, O, P> PureParser<'a, I> for MapErr<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: Fn(P::Error) -> O,
    P: Parser<'a, I> + PureParser<'a, I> + Sized,
{
    fn output_len(output: Self::Output) -> usize {
        P::output_len(output)
    }
}

/// [Parser] created by [Parser::map_iter]
#[derive(Debug)]
pub struct MapIter<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: Fn(&mut Iter<'a, I, &P>) -> O,
    P: Parser<'a, I> + Sized,
{
    pub(super) parser: P,
    pub(super) map: F,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, F, O, P> Clone for MapIter<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: Clone + Fn(&mut Iter<'a, I, &P>) -> O,
    P: Clone + Parser<'a, I> + Sized,
{
    fn clone(&self) -> Self {
        Self {
            parser: self.parser.clone(),
            map: self.map.clone(),
            _marker: PhantomData,
        }
    }
}
impl<'a, I, F, O, P> Copy for MapIter<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: Clone + Copy + Fn(&mut Iter<'a, I, &P>) -> O,
    P: Clone + Copy + Parser<'a, I> + Sized,
{
}
impl<'a, I, F, O, P> Parser<'a, I> for MapIter<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: Fn(&mut Iter<'a, I, &P>) -> O,
    P: Parser<'a, I> + Sized,
{
    type Error = Infallible;
    type Output = O;

    fn parse(&self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        let mut iter = Iter {
            input,
            parser: &self.parser,
            _marker: PhantomData,
        };

        Ok(ParserOutput {
            output: (self.map)(&mut iter),
            next: iter.input,
            _marker: PhantomData,
        })
    }
}

/// [Parser] created by [Parser::or]
#[derive(Debug)]
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
impl<'a, I, O, L, R> Clone for Or<'a, I, O, L, R>
where
    I: Parsable<'a>,
    L: Clone + Parser<'a, I, Output = O>,
    R: Clone + Parser<'a, I, Output = O>,
{
    fn clone(&self) -> Self {
        Self {
            l: self.l.clone(),
            r: self.r.clone(),
            _marker: PhantomData,
        }
    }
}
impl<'a, I, O, L, R> Copy for Or<'a, I, O, L, R>
where
    I: Parsable<'a>,
    L: Clone + Copy + Parser<'a, I, Output = O>,
    R: Clone + Copy + Parser<'a, I, Output = O>,
{
}
impl<'a, I, O, L, R> Parser<'a, I> for Or<'a, I, O, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I, Output = O>,
    R: Parser<'a, I, Output = O>,
{
    type Error = R::Error;
    type Output = O;

    fn parse(&self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        if let Ok(output) = self.l.parse(input) {
            Ok(output)
        } else {
            self.r.parse(input)
        }
    }
}
// SAFETY: Assuming the left parser implements [PureParser] correctly, this should be fine.
unsafe impl<'a, I, O, L, R> PureParser<'a, I> for Or<'a, I, O, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I, Output = O> + PureParser<'a, I> + Sized,
    R: Parser<'a, I, Output = O> + PureParser<'a, I> + Sized,
{
    fn output_len(output: Self::Output) -> usize {
        L::output_len(output)
    }
}

/// [Parser] created by [PureParser::repeated]
#[derive(Clone, Copy, Debug)]
pub struct Repeated<'a, I, P>
where
    I: Parsable<'a>,
    P: Clone + PureParser<'a, I> + Sized,
{
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, P> Parser<'a, I> for Repeated<'a, I, P>
where
    I: Parsable<'a>,
    P: Clone + PureParser<'a, I> + Sized,
{
    type Error = Infallible;
    type Output = I;

    fn parse(&self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        // SAFETY: should not panic if all parsers are pure
        let (output, next) = input.split_at(
            Iter {
                input,
                parser: &self.parser,
                _marker: PhantomData,
            }
            .map(P::output_len)
            .sum::<usize>(),
        );

        Ok(ParserOutput::new(next, output))
    }
}
unsafe impl<'a, I, P> PureParser<'a, I> for Repeated<'a, I, P>
where
    I: Parsable<'a>,
    P: Clone + PureParser<'a, I> + Sized,
{
    fn output_len(output: Self::Output) -> usize {
        output.items_len()
    }
}

/// [Parser] created by [Parser::then]
#[derive(Debug)]
pub struct Then<'a, I, E, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I, Error = E>,
    R: Parser<'a, I, Error = E>,
{
    pub(super) l: L,
    pub(super) r: R,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, E, L, R> Clone for Then<'a, I, E, L, R>
where
    I: Parsable<'a>,
    L: Clone + Parser<'a, I, Error = E>,
    R: Clone + Parser<'a, I, Error = E>,
{
    fn clone(&self) -> Self {
        Self {
            l: self.l.clone(),
            r: self.r.clone(),
            _marker: PhantomData,
        }
    }
}
impl<'a, I, E, L, R> Copy for Then<'a, I, E, L, R>
where
    I: Parsable<'a>,
    L: Clone + Copy + Parser<'a, I, Error = E>,
    R: Clone + Copy + Parser<'a, I, Error = E>,
{
}
impl<'a, I, E, L, R> Parser<'a, I> for Then<'a, I, E, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I, Error = E>,
    R: Parser<'a, I, Error = E>,
{
    type Error = E;
    type Output = (L::Output, R::Output);

    fn parse(&self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        let items = input.items();
        let ParserOutput {
            next: items,
            output: l,
            ..
        } = self.l.parse(I::recover(items))?;
        let ParserOutput {
            next: items,
            output: r,
            ..
        } = self.r.parse(items)?;

        Ok(ParserOutput::new(items, (l, r)))
    }
}
// SAFETY: should be fine if both parsers are pure
unsafe impl<'a, I, E, L, R> PureParser<'a, I> for Then<'a, I, E, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I> + PureParser<'a, I, Error = E> + Sized,
    R: Parser<'a, I> + PureParser<'a, I, Error = E> + Sized,
{
    fn output_len((l, r): Self::Output) -> usize {
        [L::output_len(l), R::output_len(r)]
            .into_iter()
            .sum::<usize>()
    }
}

/// [Parser] created by [Parser::then_ignore]
#[derive(Debug)]
pub struct ThenIgnore<'a, I, E, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I, Error = E>,
    R: Parser<'a, I, Error = E>,
{
    pub(super) l: L,
    pub(super) r: R,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, E, L, R> Clone for ThenIgnore<'a, I, E, L, R>
where
    I: Parsable<'a>,
    L: Clone + Parser<'a, I, Error = E>,
    R: Clone + Parser<'a, I, Error = E>,
{
    fn clone(&self) -> Self {
        Self {
            l: self.l.clone(),
            r: self.r.clone(),
            _marker: PhantomData,
        }
    }
}
impl<'a, I, E, L, R> Copy for ThenIgnore<'a, I, E, L, R>
where
    I: Parsable<'a>,
    L: Clone + Copy + Parser<'a, I, Error = E>,
    R: Clone + Copy + Parser<'a, I, Error = E>,
{
}
impl<'a, I, E, L, R> Parser<'a, I> for ThenIgnore<'a, I, E, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I, Error = E>,
    R: Parser<'a, I, Error = E>,
{
    type Error = E;
    type Output = L::Output;

    fn parse(&self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        let ParserOutput {
            next: input,
            output,
            ..
        } = self.l.parse(input)?;
        let ParserOutput { next: input, .. } = self.r.parse(input)?;

        Ok(ParserOutput::new(input, output))
    }
}
