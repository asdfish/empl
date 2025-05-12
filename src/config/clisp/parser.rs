//! Parser combinators

mod adapter;
pub use adapter::*;
mod token;
pub use token::*;

use {
    std::{
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
    I: Parsable<'a> ,
{
    type Error;
    type Output;

    fn parse(
        self,
        _: I,
    ) -> Result<ParserOutput<'a, I, Self::Output>, ParserError<I::Item, Self::Error>>;

    /// Pick either heterogeneous parsers with an output of `Either<Self::Output, R::Output>` and an error of `R::Error`.
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

    /// Pick either homogeneous parsers with an output of [Self::Output] and an error of `R::Error`.
    ///
    /// # Examples
    /// ```
    /// # use empl::{config::clisp::parser::{Parser, ParserOutput, ParserError, Just}, either::Either};
    /// let a_or_b = Just('a').or(Just('b'));
    /// assert_eq!(a_or_b.parse("a"), Ok(ParserOutput::new("", 'a')));
    /// assert_eq!(a_or_b.parse("b"), Ok(ParserOutput::new("", 'b')));
    /// assert_eq!(a_or_b.parse("c"), Err(ParserError::Match { expected: 'b', found: 'c' }));
    /// ```
    fn or<R>(self, r: R) -> Or<'a, I, Self::Output, Self, R>
    where R: Parser<'a, I, Output = Self::Output> {
        Or {
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParserOutput<'a, I, O>
where
    I: Parsable<'a> ,
{
    pub next: I,
    pub output: O,
    _marker: PhantomData<&'a ()>,
}
impl<'a, I, O> PartialEq<O> for ParserOutput<'a, I, O>
where
    I: Parsable<'a> ,
    O: PartialEq,
{
    fn eq(&self, r: &O) -> bool {
        self.output.eq(r)
    }
}
impl<'a, I, O> ParserOutput<'a, I, O>
where
    I: Parsable<'a> ,
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
