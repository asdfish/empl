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
    std::marker::PhantomData,
};

/// [Parser] created by [PureParser::as_slice]
#[derive(Clone, Copy, Debug)]
pub struct AsSlice<'a, I, P>
where
    I: Parsable<'a>,
    P: PureParser<'a, I>,
{
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, P> Parser<'a, I> for AsSlice<'a, I, P>
where
    I: Parsable<'a>,
    P: PureParser<'a, I>,
{
    type Output = I;

    fn parse(&self, input: I) -> Option<ParserOutput<'a, I, Self::Output>> {
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
    P: PureParser<'a, I>,
{
    fn output_len(output: Self::Output) -> usize {
        I::items_len(output)
    }
}

/// [Parser] created by [Parser::co_flatten]
#[derive(Clone, Copy, Debug)]
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

    fn parse(&self, input: I) -> Option<ParserOutput<'a, I, Self::Output>> {
        Some(
            self.parser
                .parse(input)
                .map(|output| output.map_output(Some))
                .unwrap_or_else(|| ParserOutput::new(input, None)),
        )
    }
}

/// [Parser] created by [Parser::delimited_by]
#[derive(Debug)]
pub struct DelimitedBy<'a, I, L, P, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I>,
    P: Parser<'a, I>,
    R: Parser<'a, I>,
{
    pub(super) l: L,
    pub(super) parser: P,
    pub(super) r: R,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, L, P, R> Clone for DelimitedBy<'a, I, L, P, R>
where
    I: Parsable<'a>,
    L: Clone + Parser<'a, I>,
    P: Clone + Parser<'a, I>,
    R: Clone + Parser<'a, I>,
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
impl<'a, I, L, P, R> Copy for DelimitedBy<'a, I, L, P, R>
where
    I: Parsable<'a>,
    L: Clone + Copy + Parser<'a, I>,
    P: Clone + Copy + Parser<'a, I>,
    R: Clone + Copy + Parser<'a, I>,
{
}
impl<'a, I, L, P, R> Parser<'a, I> for DelimitedBy<'a, I, L, P, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I>,
    P: Parser<'a, I>,
    R: Parser<'a, I>,
{
    type Output = P::Output;

    fn parse(&self, input: I) -> Option<ParserOutput<'a, I, Self::Output>> {
        let ParserOutput { next: input, .. } = self.l.parse(input)?;
        let ParserOutput {
            next: input,
            output,
            ..
        } = self.parser.parse(input)?;
        let ParserOutput { next: input, .. } = self.r.parse(input)?;

        Some(ParserOutput::new(input, output))
    }
}

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
    type Output = Either<L::Output, R::Output>;

    fn parse(&self, input: I) -> Option<ParserOutput<'a, I, Self::Output>> {
        if let Some(po) = self.l.parse(input).map(|po| po.map_output(Either::Left)) {
            return Some(po);
        }

        self.r.parse(input).map(|po| po.map_output(Either::Right))
    }
}
// SAFETY: both parsers must be pure
unsafe impl<'a, I, L, R> PureParser<'a, I> for EitherOr<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I> + PureParser<'a, I>,
    R: Parser<'a, I> + PureParser<'a, I>,
{
    fn output_len(output: Self::Output) -> usize {
        match output {
            Either::Left(output) => L::output_len(output),
            Either::Right(output) => R::output_len(output),
        }
    }
}

/// [Parser] created by [Parser::filter]
#[derive(Clone, Copy, Debug)]
pub struct Filter<'a, F, I, P>
where
    I: Parsable<'a>,
    F: Fn(&P::Output) -> bool,
    P: Parser<'a, I>,
{
    pub(super) parser: P,
    pub(super) predicate: F,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, F, I, P> Parser<'a, I> for Filter<'a, F, I, P>
where
    I: Parsable<'a>,
    F: Fn(&P::Output) -> bool,
    P: Parser<'a, I>,
{
    type Output = P::Output;

    fn parse(&self, input: I) -> Option<ParserOutput<'a, I, Self::Output>> {
        self.parser
            .parse(input)
            .filter(|ParserOutput { output, .. }| (self.predicate)(output))
    }
}
unsafe impl<'a, F, I, P> PureParser<'a, I> for Filter<'a, F, I, P>
where
    I: Parsable<'a>,
    F: Fn(&P::Output) -> bool,
    P: Parser<'a, I> + PureParser<'a, I>,
{
    fn output_len(output: Self::Output) -> usize {
        P::output_len(output)
    }
}

/// [Parser] created by [Parser::filter_map]
#[derive(Debug)]
pub struct FilterMap<'a, I, M, P, T>
where
    I: Parsable<'a>,
    M: Fn(P::Output) -> Option<T>,
    P: Parser<'a, I>,
{
    pub(super) map: M,
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, M, P, T> Clone for FilterMap<'a, I, M, P, T>
where
    I: Parsable<'a>,
    M: Clone + Fn(P::Output) -> Option<T>,
    P: Clone + Parser<'a, I>,
{
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
            parser: self.parser.clone(),
            _marker: PhantomData,
        }
    }
}
impl<'a, I, M, P, T> Copy for FilterMap<'a, I, M, P, T>
where
    I: Parsable<'a>,
    M: Clone + Copy + Fn(P::Output) -> Option<T>,
    P: Clone + Copy + Parser<'a, I>,
{
}
impl<'a, I, M, P, T> Parser<'a, I> for FilterMap<'a, I, M, P, T>
where
    I: Parsable<'a>,
    M: Fn(P::Output) -> Option<T>,
    P: Parser<'a, I>,
{
    type Output = T;

    fn parse(&self, input: I) -> Option<ParserOutput<'a, I, Self::Output>> {
        self.parser.parse(input)?.map_output(&self.map).transpose()
    }
}

#[derive(Debug)]
pub struct Flatten<'a, I, P, T>
where
    I: Parsable<'a>,
    P: Parser<'a, I, Output = Option<T>>,
{
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, P, T> Clone for Flatten<'a, I, P, T>
where
    I: Parsable<'a>,
    P: Clone + Parser<'a, I, Output = Option<T>>,
{
    fn clone(&self) -> Self {
        Self {
            parser: self.parser.clone(),
            _marker: PhantomData,
        }
    }
}
impl<'a, I, P, T> Copy for Flatten<'a, I, P, T>
where
    I: Parsable<'a>,
    P: Clone + Copy + Parser<'a, I, Output = Option<T>>,
{
}
impl<'a, I, P, T> Parser<'a, I> for Flatten<'a, I, P, T>
where
    I: Parsable<'a>,
    P: Parser<'a, I, Output = Option<T>>,
{
    type Output = T;

    fn parse(&self, input: I) -> Option<ParserOutput<'a, I, Self::Output>> {
        self.parser.parse(input).and_then(ParserOutput::transpose)
    }
}

/// [Parser] created by [Parser::fold]
#[derive(Debug)]
pub struct Fold<'a, A, AF, F, I, P>
where
    AF: Fn() -> A,
    I: Parsable<'a>,
    F: Fn(A, I, P::Output) -> Option<A>,
    P: Clone + Parser<'a, I>,
{
    pub(super) fold: F,
    pub(super) parser: P,
    pub(super) start: AF,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, A, AF, F, I, P> Clone for Fold<'a, A, AF, F, I, P>
where
    I: Parsable<'a>,
    AF: Clone + Fn() -> A,
    F: Clone + Fn(A, I, P::Output) -> Option<A>,
    P: Clone + Parser<'a, I>,
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
impl<'a, A, AF, F, I, P> Copy for Fold<'a, A, AF, F, I, P>
where
    I: Parsable<'a>,
    AF: Clone + Copy + Fn() -> A,
    F: Clone + Copy + Fn(A, I, P::Output) -> Option<A>,
    P: Clone + Copy + Parser<'a, I>,
{
}
impl<'a, A, AF, F, I, P> Parser<'a, I> for Fold<'a, A, AF, F, I, P>
where
    AF: Fn() -> A,
    I: Parsable<'a>,
    F: Fn(A, I, P::Output) -> Option<A>,
    P: Clone + Parser<'a, I>,
{
    type Output = A;

    fn parse(&self, input: I) -> Option<ParserOutput<'a, I, Self::Output>> {
        let mut accum = (self.start)();
        let mut items = input;

        while let Some(ParserOutput { next, output, .. }) = self.parser.parse(items) {
            let Some((slice, _)) = input.split_at_checked(input.items_len() - next.items_len())
            else {
                break;
            };
            accum = (self.fold)(accum, slice, output)?;

            items = next;
        }

        Some(ParserOutput::new(items, accum))
    }
}

/// [Parser] created by [Parser::ignore_then]
#[derive(Debug)]
pub struct IgnoreThen<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I>,
    R: Parser<'a, I>,
{
    pub(super) l: L,
    pub(super) r: R,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, L, R> Clone for IgnoreThen<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Clone + Parser<'a, I>,
    R: Clone + Parser<'a, I>,
{
    fn clone(&self) -> Self {
        Self {
            l: self.l.clone(),
            r: self.r.clone(),
            _marker: PhantomData,
        }
    }
}
impl<'a, I, L, R> Copy for IgnoreThen<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Clone + Copy + Parser<'a, I>,
    R: Clone + Copy + Parser<'a, I>,
{
}
impl<'a, I, L, R> Parser<'a, I> for IgnoreThen<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I>,
    R: Parser<'a, I>,
{
    type Output = R::Output;

    fn parse(&self, input: I) -> Option<ParserOutput<'a, I, Self::Output>> {
        let ParserOutput { next: input, .. } = self.l.parse(input)?;
        let ParserOutput {
            next: input,
            output,
            ..
        } = self.r.parse(input)?;

        Some(ParserOutput::new(input, output))
    }
}

/// [Iterator] for [Parser]s
#[derive(Clone, Copy, Debug)]
pub struct Iter<'a, I, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
{
    pub(super) input: I,
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a ()>,
}
impl<'a, I, P> Iterator for Iter<'a, I, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
{
    type Item = P::Output;

    fn next(&mut self) -> Option<P::Output> {
        self.parser
            .parse(self.input)
            .map(|ParserOutput { next, output, .. }| {
                self.input = next;
                output
            })
    }
}

/// [Parser] created by [Parser::map]
#[derive(Debug)]
pub struct Map<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: Fn(P::Output) -> O,
    P: Parser<'a, I>,
{
    pub(super) map: F,
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, O, F, P> Clone for Map<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: Clone + Fn(P::Output) -> O,
    P: Clone + Parser<'a, I>,
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
    P: Clone + Copy + Parser<'a, I>,
{
}
impl<'a, I, O, F, P> Parser<'a, I> for Map<'a, I, F, O, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
    F: Fn(P::Output) -> O,
{
    type Output = O;

    fn parse(&self, input: I) -> Option<ParserOutput<'a, I, Self::Output>> {
        self.parser
            .parse(input)
            .map(move |output| output.map_output(&self.map))
    }
}

/// [Parser] created by [Parser::map_iter]
#[derive(Debug)]
pub struct MapIter<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: Fn(&mut Iter<'a, I, &P>) -> O,
    P: Parser<'a, I>,
{
    pub(super) parser: P,
    pub(super) map: F,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, F, O, P> Clone for MapIter<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: Clone + Fn(&mut Iter<'a, I, &P>) -> O,
    P: Clone + Parser<'a, I>,
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
    P: Clone + Copy + Parser<'a, I>,
{
}
impl<'a, I, F, O, P> Parser<'a, I> for MapIter<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: Fn(&mut Iter<'a, I, &P>) -> O,
    P: Parser<'a, I>,
{
    type Output = O;

    fn parse(&self, input: I) -> Option<ParserOutput<'a, I, Self::Output>> {
        let mut iter = Iter {
            input,
            parser: &self.parser,
            _marker: PhantomData,
        };

        Some(ParserOutput {
            output: (self.map)(&mut iter),
            next: iter.input,
            _marker: PhantomData,
        })
    }
}

/// [Parser] created by [Parser::maybe].
#[derive(Clone, Copy, Debug)]
pub struct Maybe<'a, I, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
{
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, P> Parser<'a, I> for Maybe<'a, I, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
{
    type Output = Option<P::Output>;

    fn parse(&self, input: I) -> Option<ParserOutput<'a, I, Self::Output>> {
        Some(
            self.parser
                .parse(input)
                .map(|output| output.map_output(Some))
                .unwrap_or_else(|| ParserOutput::new(input, None)),
        )
    }
}
unsafe impl<'a, I, P> PureParser<'a, I> for Maybe<'a, I, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I> + PureParser<'a, I>,
{
    fn output_len(output: Self::Output) -> usize {
        output.map(P::output_len).unwrap_or(0)
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
    type Output = O;

    fn parse(&self, input: I) -> Option<ParserOutput<'a, I, Self::Output>> {
        self.l.parse(input).or_else(|| self.r.parse(input))
    }
}
// SAFETY: Assuming the left parser implements [PureParser] correctly, this should be fine.
unsafe impl<'a, I, O, L, R> PureParser<'a, I> for Or<'a, I, O, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I, Output = O> + PureParser<'a, I>,
    R: Parser<'a, I, Output = O> + PureParser<'a, I>,
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
    P: Clone + PureParser<'a, I>,
{
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, P> Parser<'a, I> for Repeated<'a, I, P>
where
    I: Parsable<'a>,
    P: Clone + PureParser<'a, I>,
{
    type Output = I;

    fn parse(&self, input: I) -> Option<ParserOutput<'a, I, Self::Output>> {
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

        Some(ParserOutput::new(next, output))
    }
}
unsafe impl<'a, I, P> PureParser<'a, I> for Repeated<'a, I, P>
where
    I: Parsable<'a>,
    P: Clone + PureParser<'a, I>,
{
    fn output_len(output: Self::Output) -> usize {
        output.items_len()
    }
}

/// [Parser] created by [Parser::then]
#[derive(Debug)]
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
impl<'a, I, L, R> Clone for Then<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Clone + Parser<'a, I>,
    R: Clone + Parser<'a, I>,
{
    fn clone(&self) -> Self {
        Self {
            l: self.l.clone(),
            r: self.r.clone(),
            _marker: PhantomData,
        }
    }
}
impl<'a, I, L, R> Copy for Then<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Clone + Copy + Parser<'a, I>,
    R: Clone + Copy + Parser<'a, I>,
{
}
impl<'a, I, L, R> Parser<'a, I> for Then<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I>,
    R: Parser<'a, I>,
{
    type Output = (L::Output, R::Output);

    fn parse(&self, input: I) -> Option<ParserOutput<'a, I, Self::Output>> {
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

        Some(ParserOutput::new(items, (l, r)))
    }
}
// SAFETY: should be fine if both parsers are pure
unsafe impl<'a, I, L, R> PureParser<'a, I> for Then<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I> + PureParser<'a, I>,
    R: Parser<'a, I> + PureParser<'a, I>,
{
    fn output_len((l, r): Self::Output) -> usize {
        [L::output_len(l), R::output_len(r)]
            .into_iter()
            .sum::<usize>()
    }
}

/// [Parser] created by [Parser::then_ignore]
#[derive(Clone, Copy, Debug)]
pub struct ThenIgnore<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I>,
    R: Parser<'a, I>,
{
    pub(super) l: L,
    pub(super) r: R,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, L, R> Parser<'a, I> for ThenIgnore<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I>,
    R: Parser<'a, I>,
{
    type Output = L::Output;

    fn parse(&self, input: I) -> Option<ParserOutput<'a, I, Self::Output>> {
        let ParserOutput {
            next: input,
            output,
            ..
        } = self.l.parse(input)?;
        let ParserOutput { next: input, .. } = self.r.parse(input)?;

        Some(ParserOutput::new(input, output))
    }
}
