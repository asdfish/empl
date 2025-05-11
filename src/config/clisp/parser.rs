//! Parser combinators

use {
    crate::either::Either,
    std::{error::Error, marker::PhantomData, slice, str},
};

pub trait Parsable<'a> {
    type Item;
    type Iter: Iterator<Item = Self::Item>;

    fn item_len(_: Self::Item) -> usize;
    fn items(&'a self) -> Self::Iter;
    fn recover(_: Self::Iter) -> &'a Self;
}
impl<'a> Parsable<'a> for str {
    type Item = char;
    type Iter = str::Chars<'a>;

    fn item_len(ch: Self::Item) -> usize {
        ch.len_utf8()
    }
    fn items(&'a self) -> Self::Iter {
        self.chars()
    }
    fn recover(chars: Self::Iter) -> &'a str {
        chars.as_str()
    }
}
impl<'a, T> Parsable<'a> for [T]
where
    T: 'a,
{
    type Item = &'a T;
    type Iter = slice::Iter<'a, T>;

    fn item_len(_: Self::Item) -> usize {
        1
    }
    fn items(&'a self) -> Self::Iter {
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
        _: &'a I,
    ) -> Result<ParserOutput<'a, I, Self::Output>, ParserError<I::Item, Self::Error>>;

    /// Chain two parsers together with an output of `(Self::Output, R::Output)` and an error of `Either<Self::Error, R::Error>`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::config::clisp::parser::{Parser, ParserOutput, ParserError, Just};
    /// assert_eq!(Just('h').then(Just('i')).parse("hi"), Ok(ParserOutput { next: "", output: ('h', 'i') }));
    /// assert_eq!(Just('h').then(Just('i')).parse("ho"), Err(ParserError::Match { expected: 'i', found: 'o' }));
    /// ```
    fn then<R>(self, r: R) -> Then<'a, I, Self, R>
    where R: Parser<'a, I> {
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
/// assert_eq!(Just('h').parse("hello"), Ok(ParserOutput { next: "ello", output: 'h' }));
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
    type Error = ();
    type Output = T;

    fn parse(
        self,
        input: &'a I,
    ) -> Result<ParserOutput<'a, I, Self::Output>, ParserError<I::Item, Self::Error>> {
        let mut items = input.items();

        match items.next().ok_or(ParserError::Eof)? {
            item if item == self.0 => Ok(ParserOutput {
                next: I::recover(items),
                output: item,
            }),
            item => Err(ParserError::Match {
                expected: self.0,
                found: item,
            }),
        }
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
        input: &'a I,
    ) -> Result<ParserOutput<'a, I, Self::Output>, ParserError<I::Item, Self::Error>> {
        let items = input.items();
        let ParserOutput {
            next: items,
            output: l,
        } = self
            .l
            .parse(I::recover(items))
            .map_err(|err| err.map_custom(Either::Left))?;
        let ParserOutput {
            next: items,
            output: r,
        } = self
            .r
            .parse(items)
            .map_err(|err| err.map_custom(Either::Right))?;

        Ok(ParserOutput {
            next: items,
            output: (l, r),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParserOutput<'a, I, O>
where
    I: Parsable<'a> + ?Sized,
{
    pub next: &'a I,
    pub output: O,
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
