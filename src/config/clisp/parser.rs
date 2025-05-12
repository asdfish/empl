//! Parser combinators

pub mod adapter;
pub mod token;

use {
    crate::config::clisp::parser::adapter::*,
    std::{marker::PhantomData, ops::Deref, slice, str},
};

pub trait Parsable<'a>: Copy + Deref + Sized {
    type Item;
    type Iter: DoubleEndedIterator + Iterator<Item = Self::Item>;

    fn items(self) -> Self::Iter;
    fn recover(_: Self::Iter) -> Self;
}
impl<'a> Parsable<'a> for &'a str {
    type Item = char;
    type Iter = str::Chars<'a>;

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

    fn items(self) -> Self::Iter {
        self.iter()
    }
    fn recover(items: Self::Iter) -> &'a [T] {
        items.as_slice()
    }
}

pub trait Parser<'a, I>: Sized
where
    I: Parsable<'a>,
{
    type Error;
    type Output;

    fn parse(self, _: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error>;

    /// Pick either heterogeneous parsers with an output of `Either<Self::Output, R::Output>` and an error of `R::Error`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::{config::clisp::parser::{Parser, ParserOutput, ParserError, token::{Just, Sequence}}, either::Either};
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

    /// Convert a `Result<T, E>` to a `T` by making the error part of the parser.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::{config::clisp::parser::{Parser, ParserOutput, ParserError, token::Sequence}, either::Either};
    /// # use std::str::FromStr;
    /// let answer_to_life = Sequence::new("42").map(u32::from_str).flatten_err();
    /// assert_eq!(answer_to_life.parse("42"), Ok(ParserOutput::new("", 42)));
    /// assert_eq!(answer_to_life.parse("1"), Err(Either::Left(ParserError::Match { expected: '4', found: '1' })));
    /// ```
    fn flatten_err<E, O>(self) -> FlattenErr<'a, I, E, O, Self>
    where
        Self: Parser<'a, I, Output = Result<O, E>>,
    {
        FlattenErr {
            parser: self,
            _marker: PhantomData,
        }
    }

    /// Transform the output of the current [Parser].
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::{config::clisp::parser::{Parser, ParserOutput, ParserError, token::{Any, Just}}, either::Either};
    /// let lowercase = Any::new().map(|ch: char| ch.to_ascii_lowercase());
    /// assert_eq!(lowercase.parse("a"), Ok(ParserOutput::new("", 'a')));
    /// assert_eq!(lowercase.parse("A"), Ok(ParserOutput::new("", 'a')));
    /// ```
    fn map<F, O>(self, map: F) -> Map<F, Self>
    where
        F: FnOnce(Self::Output) -> O,
    {
        Map {
            parser: self,
            map,
        }
    }

    /// Repeat the current parser to enable some operations that can only be executed on repeating parsers.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::config::clisp::parser::{Parser, ParserOutput, ParserError, token::Just};
    /// let count_a = Just('a').map_iter(|iter| iter.count());
    /// assert_eq!(count_a.parse("aaa"), Ok(ParserOutput::new("", 3)));
    /// assert_eq!(count_a.parse("aaabbb"), Ok(ParserOutput::new("bbb", 3)));
    /// ```
    fn map_iter<F, O>(self, map: F) -> MapIter<'a, I, F, O, Self>
    where
        Self: Clone,
        F: FnOnce(&mut Iter<'a, I, Self>) -> O,
    {
        MapIter {
            parser: self,
            map,
            _marker: PhantomData,
        }
    }

    /// Pick either homogeneous parsers with an output of [Self::Output] and an error of `R::Error`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::{config::clisp::parser::{Parser, ParserOutput, ParserError, token::Just}, either::Either};
    /// let a_or_b = Just('a').or(Just('b'));
    /// assert_eq!(a_or_b.parse("a"), Ok(ParserOutput::new("", 'a')));
    /// assert_eq!(a_or_b.parse("b"), Ok(ParserOutput::new("", 'b')));
    /// assert_eq!(a_or_b.parse("c"), Err(ParserError::Match { expected: 'b', found: 'c' }));
    /// ```
    fn or<R>(self, r: R) -> Or<'a, I, Self::Output, Self, R>
    where
        R: Parser<'a, I, Output = Self::Output>,
    {
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
    /// # use empl::{config::clisp::parser::{Parser, ParserOutput, ParserError, token::Just}, either::Either};
    /// assert_eq!(Just('h').then(Just('i')).parse("hi"), Ok(ParserOutput::new("", ('h', 'i'))));
    /// assert_eq!(Just('h').then(Just('i')).parse("ho"), Err(Either::Right(ParserError::Match { expected: 'i', found: 'o' })));
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

    /// Set the output of a parser
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::config::clisp::parser::{Parser, ParserOutput, ParserError, token::{Any, Just}};
    /// let is_a = Just('a').to(true).or(Any::new().to(false));
    /// assert_eq!(is_a.parse("a"), Ok(ParserOutput::new("", true)));
    /// assert_eq!(is_a.parse("b"), Ok(ParserOutput::new("", false)));
    /// ```
    fn to<T>(self, to: T) -> To<'a, I, Self, T> {
        To {
            parser: self,
            to,
            _marker: PhantomData,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParserOutput<'a, I, O>
where
    I: Parsable<'a>,
{
    pub next: I,
    pub output: O,
    _marker: PhantomData<&'a ()>,
}
impl<'a, I, O> PartialEq<O> for ParserOutput<'a, I, O>
where
    I: Parsable<'a>,
    O: PartialEq,
{
    fn eq(&self, r: &O) -> bool {
        self.output.eq(r)
    }
}
impl<'a, I, O> ParserOutput<'a, I, O>
where
    I: Parsable<'a>,
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
impl<'a, I, E, T> ParserOutput<'a, I, Result<T, E>>
where
    I: Parsable<'a>,
{
    pub fn transpose(self) -> Result<ParserOutput<'a, I, T>, E> {
        match self.output {
            Ok(output) => Ok(ParserOutput {
                next: self.next,
                output,
                _marker: PhantomData,
            }),
            Err(err) => Err(err),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EofError;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParserError<T> {
    Eof(EofError),
    Match { expected: T, found: T },
}
