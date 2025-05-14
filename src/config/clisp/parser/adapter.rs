use {
    crate::{
        config::clisp::parser::{Parsable, Parser, ParserError, ParserOutput, PureParser},
        either::Either,
    },
    std::{convert::Infallible, marker::PhantomData},
};

/// [Parser] created by [Parser::co_flatten_err]
#[derive(Clone, Copy, Debug)]
pub struct CoFlattenErr<'a, I, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I>
{
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, P> Parser<'a, I> for CoFlattenErr<'a, I, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
{
    type Error = Infallible;
    type Output = Result<P::Output, P::Error>;

    fn parse(self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        Ok(self.parser.parse(input)
            .map(|output| output.map_output(Ok))
            .map_err(Err)
            .unwrap_or_else(|err| ParserOutput::new(input, err)))
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
    type Error = R::Error;
    type Output = Either<L::Output, R::Output>;

    fn parse(self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
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
    F: FnOnce(&P::Output) -> bool,
    P: Parser<'a, I>,
{
    pub(super) filter: F,
    pub(super) parser: P,
    pub(super) rule: &'static str,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, F, I, P> Parser<'a, I> for Filter<'a, F, I, P>
where
    F: FnOnce(&P::Output) -> bool,
    I: Parsable<'a>,
    P: Parser<'a, I>,
{
    type Error = Either<P::Error, ParserError<P::Output>>;
    type Output = P::Output;

    fn parse(self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        let output = self.parser.parse(input).map_err(Either::Left)?;

        if (self.filter)(&output.output) {
            Ok(output)
        } else {
            Err(Either::Right(ParserError::Rule {
                item: output.output,
                rule: self.rule,
            }))
        }
    }
}
// SAFETY: should be safe if `P` is a [PureParser]
unsafe impl<'a, F, I, P> PureParser<'a, I> for Filter<'a, F, I, P>
where
    F: FnOnce(&P::Output) -> bool,
    I: Parsable<'a>,
    P: Parser<'a, I> + PureParser<'a, I>,
{
    fn output_len(output: Self::Output) -> usize {
        P::output_len(output)
    }
}

/// [Parser] created by [Parser::flatten_err]
#[derive(Debug)]
pub struct FlattenErr<'a, I, E, O, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I, Output = Result<O, E>>,
{
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, E, O, P> Clone for FlattenErr<'a, I, E, O, P>
where
    I: Parsable<'a>,
    P: Clone + Parser<'a, I, Output = Result<O, E>>
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
    P: Clone + Copy + Parser<'a, I, Output = Result<O, E>>
{}
impl<'a, I, E, O, P> Parser<'a, I> for FlattenErr<'a, I, E, O, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I, Output = Result<O, E>>,
{
    type Error = Either<P::Error, E>;
    type Output = O;

    fn parse(self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        self.parser
            .parse(input)
            .map_err(Either::Left)
            .and_then(|output| output.transpose().map_err(Either::Right))
    }
}

/// [Iterator] for [Parser]s
#[derive(Clone, Copy, Debug)]
pub struct Iter<'a, I, P>
where
    I: Parsable<'a>,
    P: Clone + Parser<'a, I>,
{
    input: I,
    parser: P,
    _marker: PhantomData<&'a ()>,
}
impl<'a, I, P> Iterator for Iter<'a, I, P>
where
    I: Parsable<'a>,
    P: Clone + Parser<'a, I>,
{
    type Item = P::Output;

    fn next(&mut self) -> Option<P::Output> {
        self.parser
            .clone()
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
    F: FnOnce(P::Output) -> O,
    P: Parser<'a, I>,
{
    pub(super) map: F,
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, O, F, P> Clone for Map<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: Clone + FnOnce(P::Output) -> O,
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
    F: Clone + Copy + FnOnce(P::Output) -> O,
    P: Clone + Copy + Parser<'a, I>,
{}
impl<'a, I, O, F, P> Parser<'a, I> for Map<'a, I, F, O, P>
where
    I: Parsable<'a>,
    P: Parser<'a, I>,
    F: FnOnce(P::Output) -> O,
{
    type Error = P::Error;
    type Output = O;

    fn parse(self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        self.parser
            .parse(input)
            .map(move |output| output.map_output(self.map))
    }
}

/// [Parser] created by [Parser::map_err]
#[derive(Debug)]
pub struct MapErr<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: FnOnce(P::Error) -> O,
    P: Parser<'a, I>,
{
    pub(super) map: F,
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, F, O, P> Clone for MapErr<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: Clone + FnOnce(P::Error) -> O,
    P: Clone + Parser<'a, I>
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
    F: Clone + Copy + FnOnce(P::Error) -> O,
    P: Clone + Copy + Parser<'a, I>
{}
impl<'a, I, F, O, P> Parser<'a, I> for MapErr<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: FnOnce(P::Error) -> O,
    P: Parser<'a, I>
{
    type Error = O;
    type Output = P::Output;

    fn parse(self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        self.parser.parse(input)
            .map_err(self.map)
    }
}
// SAFETY: should be safe if `P` is pure
unsafe impl<'a, I, F, O, P> PureParser<'a, I> for MapErr<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: FnOnce(P::Error) -> O,
    P: Parser<'a, I> + PureParser<'a, I>
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
    F: FnOnce(&mut Iter<'a, I, P>) -> O,
    P: Clone + Parser<'a, I>,
{
    pub(super) parser: P,
    pub(super) map: F,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, F, O, P> Clone for MapIter<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: Clone + FnOnce(&mut Iter<'a, I, P>) -> O,
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
    F: Clone + Copy + FnOnce(&mut Iter<'a, I, P>) -> O,
    P: Clone + Copy + Parser<'a, I>,
{}
impl<'a, I, F, O, P> Parser<'a, I> for MapIter<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: FnOnce(&mut Iter<'a, I, P>) -> O,
    P: Clone + Parser<'a, I>,
{
    type Error = Infallible;
    type Output = O;

    fn parse(self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        let mut iter = Iter {
            input,
            parser: self.parser,
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
{}
impl<'a, I, O, L, R> Parser<'a, I> for Or<'a, I, O, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I, Output = O>,
    R: Parser<'a, I, Output = O>,
{
    type Error = R::Error;
    type Output = O;

    fn parse(self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
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
    L: Parser<'a, I, Output = O> + PureParser<'a, I>,
    R: Parser<'a, I, Output = O> + PureParser<'a, I>,
{
    fn output_len(output: Self::Output) -> usize {
        L::output_len(output)
    }
}

/// [Parser] created by [PureParser::restore]
#[derive(Clone, Copy, Debug)]
pub struct Restore<'a, I, P>
where
    I: Parsable<'a>,
    P: PureParser<'a, I>
{
    pub(super) parser: P,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'a, I, P> Parser<'a, I> for Restore<'a, I, P>
where
    I: Parsable<'a>,
    P: PureParser<'a, I>
{
    type Error = P::Error;
    type Output = I;

    fn parse(self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        self.parser.parse(input)
            .map(|ParserOutput { output, .. }| output)
            .map(P::output_len)
            .map(|split| input.split_at(split))
            .map(|(output, next)| ParserOutput::new(next, output))
    }
}
unsafe impl<'a, I, P> PureParser<'a, I> for Restore<'a, I, P>
where
    I: Parsable<'a>,
    P: PureParser<'a, I>,
{
    fn output_len(output: Self::Output) -> usize {
        I::items_len(output)
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
    type Error = Infallible;
    type Output = I;

    fn parse(self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        // SAFETY: should not panic if all parsers are pure
        let (output, next) = input.split_at(Iter {
            input,
            parser: self.parser,
            _marker: PhantomData,
        }
            .map(P::output_len)
            .sum::<usize>());

        Ok(ParserOutput::new(
            next,
            output,
        ))
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

    fn parse(self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        let items = input.items();
        let ParserOutput {
            next: items,
            output: l,
            ..
        } = self.l.parse(I::recover(items)).map_err(Either::Left)?;
        let ParserOutput {
            next: items,
            output: r,
            ..
        } = self.r.parse(items).map_err(Either::Right)?;

        Ok(ParserOutput::new(items, (l, r)))
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

    fn parse(self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        self.parser
            .parse(input)
            .map(move |output| output.map_output(move |_| self.to))
    }
}
