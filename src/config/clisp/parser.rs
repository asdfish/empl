//! Parser combinators

use {
    crate::either::{Either, EitherOrBoth},
    std::{
        convert::Infallible,
        marker::PhantomData,
        ops::Deref,
        slice::{self, SliceIndex},
        str,
    },
};

pub trait Parsable<'a>: Copy + Deref + Sized {
    type Item;
    type Iter: Iterator<Item = Self::Item>;

    fn index<I>(self, _: I) -> Option<&'a <I as SliceIndex<Self::Target>>::Output>
    where
        I: SliceIndex<Self::Target>;
    fn item_len(_: Self::Item) -> usize;
    fn items(self) -> Self::Iter;
    fn recover(_: Self::Iter) -> Self;
}
impl<'a> Parsable<'a> for &'a str {
    type Item = char;
    type Iter = str::Chars<'a>;

    fn index<I>(self, index: I) -> Option<&'a <I as SliceIndex<Self::Target>>::Output>
    where
        I: SliceIndex<Self::Target>,
    {
        self.get(index)
    }
    fn item_len(ch: Self::Item) -> usize {
        ch.len_utf8()
    }
    fn items(self) -> Self::Iter {
        self.chars()
    }
    fn recover(chars: Self::Iter) -> &'a str {
        chars.as_str()
    }
}
impl<'a, T> Parsable<'a> for &'a [T]
where
    T: 'a,
{
    type Item = &'a T;
    type Iter = slice::Iter<'a, T>;

    fn index<I>(self, index: I) -> Option<&'a <I as SliceIndex<Self::Target>>::Output>
    where
        I: SliceIndex<Self::Target>,
    {
        self.get(index)
    }
    fn item_len(_: Self::Item) -> usize {
        1
    }
    fn items(self) -> Self::Iter {
        self.iter()
    }
    fn recover(items: Self::Iter) -> &'a [T] {
        items.as_slice()
    }
}

pub trait Parser<'a, I>: Sized
where
    I: Parsable<'a> + ?Sized,
{
    type Error;
    type Output;

    fn parse(
        self,
        _: I,
    ) -> Result<ParserOutput<'a, I, Self::Output>, ParserError<I::Item, Self::Error>>;

    /// Chain two parsers together with an output of `Either<Self::Output, R::Output>` and an error of `R::Error`.
    ///
    /// # Examples
    /// ```
    /// # use empl::{config::clisp::parser::{Parser, ParserOutput, ParserError, Just, Sequence}, either::Either};
    /// let abc = Just('a').either_or(Sequence::new("bc"));
    /// assert_eq!(abc.parse("a"), Ok(ParserOutput::new("", Either::Left('a'))));
    /// assert_eq!(abc.parse("bc"), Ok(ParserOutput::new("", Either::Right("bc"))));
    /// ```
    fn either_or<R>(self, r: R) -> EitherOr<'a, I, Self, R>
    where
        R: Parser<'a, I>,
    {
        EitherOr {
            l: self,
            r,
            _marker: PhantomData,
        }
    }

    /// Chain two parsers together with an output of `(Self::Output, R::Output)` and an error of `Either<Self::Error, R::Error>`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::config::clisp::parser::{Parser, ParserOutput, ParserError, Just};
    /// assert_eq!(Just('h').then(Just('i')).parse("hi"), Ok(ParserOutput::new("", ('h', 'i'))));
    /// assert_eq!(Just('h').then(Just('i')).parse("ho"), Err(ParserError::Match { expected: 'i', found: 'o' }));
    /// ```
    fn then<R>(self, r: R) -> Then<'a, I, Self, R>
    where
        R: Parser<'a, I>,
    {
        Then {
            l: self,
            r,
            _marker: PhantomData,
        }
    }
}

/// Identity parser that returns `self.0`
///
/// # Examples
///
/// ```
/// # use empl::config::clisp::parser::{Parser, ParserOutput, ParserError, Just};
/// assert_eq!(Just('h').parse("hello"), Ok(ParserOutput::new("ello", 'h')));
/// assert_eq!(Just('h').parse("goodbye"), Err(ParserError::Match { expected: 'h', found: 'g' }));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Just<T>(pub T)
where
    T: PartialEq;
impl<'a, I, T> Parser<'a, I> for Just<T>
where
    I: Parsable<'a, Item = T> + ?Sized,
    T: PartialEq,
{
    type Error = Infallible;
    type Output = T;

    fn parse(
        self,
        input: I,
    ) -> Result<ParserOutput<'a, I, Self::Output>, ParserError<I::Item, Self::Error>> {
        let mut items = input.items();

        match items.next().ok_or(ParserError::Eof)? {
            item if item == self.0 => Ok(ParserOutput::new(I::recover(items), item)),
            item => Err(ParserError::Match {
                expected: self.0,
                found: item,
            }),
        }
    }
}

/// Identity parser for sequences
///
/// # Examples
/// ```
/// # use empl::config::clisp::parser::{Parser, ParserOutput, ParserError, Sequence};
/// assert_eq!(Sequence::new("hello").parse("hello world"), Ok(ParserOutput::new(" world", "hello")));
/// assert_eq!(Sequence::new("hello").parse("goodbye world"), Err(ParserError::Match { expected: 'h', found: 'g' }));
/// ```
#[derive(Clone, Copy, Debug)]
pub struct Sequence<'a, T>
where
    T: Parsable<'a> + ?Sized,
    T::Item: PartialEq,
{
    seq: T,
    _marker: PhantomData<&'a ()>,
}
impl<'a, T> Sequence<'a, T>
where
    T: Parsable<'a> + ?Sized,
    T::Item: PartialEq,
{
    pub const fn new(seq: T) -> Self {
        Self {
            seq,
            _marker: PhantomData,
        }
    }
}
impl<'a, I> Parser<'a, I> for Sequence<'a, I>
where
    I: Parsable<'a> + ?Sized,
    I::Item: PartialEq,
{
    type Error = Infallible;
    type Output = I;

    fn parse(
        self,
        input: I,
    ) -> Result<ParserOutput<'a, I, Self::Output>, ParserError<I::Item, Self::Error>> {
        let mut l = self.seq.items();
        let mut r = input.items();

        while let Some(state) = EitherOrBoth::new_lazy_left(|| l.next(), || r.next()) {
            match state {
                EitherOrBoth::Left(_) => return Err(ParserError::Eof),
                EitherOrBoth::Right(_) => break,
                EitherOrBoth::Both(l, r) if l == r => continue,
                EitherOrBoth::Both(l, r) => {
                    return Err(ParserError::Match {
                        expected: l,
                        found: r,
                    })
                }
            }
        }

        Ok(ParserOutput::new(I::recover(r), self.seq))
    }
}

/// [Parser] created by [Parser::either_or]
#[derive(Clone, Copy, Debug)]
pub struct EitherOr<'a, I, L, R>
where
    I: Parsable<'a> + ?Sized,
    L: Parser<'a, I>,
    R: Parser<'a, I>,
{
    l: L,
    r: R,
    _marker: PhantomData<&'a I>,
}
impl<'a, I, L, R> Parser<'a, I> for EitherOr<'a, I, L, R>
where
    I: Parsable<'a> + ?Sized,
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

/// [Parser] created by [Parser::then]
#[derive(Clone, Copy, Debug)]
pub struct Then<'a, I, L, R>
where
    I: Parsable<'a> + ?Sized,
    L: Parser<'a, I>,
    R: Parser<'a, I>,
{
    l: L,
    r: R,
    _marker: PhantomData<&'a I>,
}
impl<'a, I, L, R> Parser<'a, I> for Then<'a, I, L, R>
where
    I: Parsable<'a> + ?Sized,
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParserOutput<'a, I, O>
where
    I: Parsable<'a> + ?Sized,
{
    pub next: I,
    pub output: O,
    _marker: PhantomData<&'a ()>,
}
impl<'a, I, O> PartialEq<O> for ParserOutput<'a, I, O>
where
    I: Parsable<'a> + ?Sized,
    O: PartialEq,
{
    fn eq(&self, r: &O) -> bool {
        self.output.eq(r)
    }
}
impl<'a, I, O> ParserOutput<'a, I, O>
where
    I: Parsable<'a> + ?Sized,
{
    pub const fn new(next: I, output: O) -> Self {
        Self {
            next,
            output,
            _marker: PhantomData,
        }
    }

    pub fn map_output<F, T>(self, f: F) -> ParserOutput<'a, I, T>
    where
        F: FnOnce(O) -> T,
    {
        ParserOutput::new(self.next, f(self.output))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParserError<T, E> {
    Custom(E),
    Eof,
    Match { expected: T, found: T },
}
impl<T, E> ParserError<T, E> {
    pub fn map_custom<F, O>(self, f: F) -> ParserError<T, O>
    where
        F: FnOnce(E) -> O,
    {
        match self {
            Self::Custom(err) => ParserError::Custom(f(err)),
            Self::Eof => ParserError::Eof,
            Self::Match { expected, found } => ParserError::Match { expected, found },
        }
    }
}
