use {
    crate::config::clisp::parser::{Parsable, Parser, ParserOutput},
    std::{convert::Infallible, marker::PhantomData},
};

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
    type Item = Result<P::Output, P::Error>;

    fn next(&mut self) -> Option<Result<P::Output, P::Error>> {
        if self.input.items().next().is_none() {
            None
        } else {
            self.parser
                .clone()
                .parse(self.input)
                .map(|ParserOutput { next, output, .. }| {
                    self.input = next;
                    output
                })
                .map(Some)
                .transpose()
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Map<'a, I, F, O, P>
where
    I: Parsable<'a>,
    F: FnOnce(&mut Iter<'a, I, P>) -> O,
    P: Clone + Parser<'a, I>,
{
    parser: P,
    map: F,
    _marker: PhantomData<&'a I>,
}
impl<'a, I, F, O, P> Parser<'a, I> for Map<'a, I, F, O, P>
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

#[derive(Clone, Copy, Debug)]
pub struct Repeated<'a, I, P>
where
    I: Parsable<'a>,
    P: Clone + Parser<'a, I>,
{
    pub(crate) parser: P,
    pub(crate) _marker: PhantomData<&'a I>,
}
impl<'a, I, P> Repeated<'a, I, P>
where
    I: Parsable<'a>,
    P: Clone + Parser<'a, I>,
{
    pub fn map<F, O>(self, map: F) -> Map<'a, I, F, O, P>
    where
        F: FnOnce(&mut Iter<'a, I, P>) -> O
    {
        Map {
            parser: self.parser,
            map,
            _marker: PhantomData,
        }
    }
    pub fn try_map<E, F, O>(self, map: F) -> TryMap<'a, I, E, F, O, P>
    where
        F: FnOnce(&mut Iter<'a, I, P>) -> Result<O, E>
    {
        TryMap {
            parser: self.parser,
            map,
            _marker: PhantomData,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TryMap<'a, I, E, F, O, P>
where
    I: Parsable<'a>,
    F: FnOnce(&mut Iter<'a, I, P>) -> Result<O, E>,
    P: Clone + Parser<'a, I>
{
    parser: P,
    map: F,
    _marker: PhantomData<&'a I>,
}
impl<'a, I, E, F, O, P> Parser<'a, I> for TryMap<'a, I, E, F, O, P>
where
    I: Parsable<'a>,
    F: FnOnce(&mut Iter<'a, I, P>) -> Result<O, E>,
    P: Clone + Parser<'a, I>
{
    type Error = E;
    type Output = O;

    fn parse(self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        let mut iter = Iter {
            input,
            parser: self.parser,
            _marker: PhantomData,
        };

        (self.map)(&mut iter)
            .map(move |output| ParserOutput {
                output,
                next: iter.input,
                _marker: PhantomData,
            })
    }
}
