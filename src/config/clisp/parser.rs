//! Parser combinators

pub mod adapter;
pub mod token;

use {
    crate::config::clisp::parser::adapter::*,
    std::{error::Error, fmt::{self, Display, Formatter}, marker::PhantomData, slice, str},
};

/// Trait for types that can be used by a [Parser].
pub trait Parsable<'a>: Copy + Sized {
    type Item;
    type Iter: Iterator<Item = Self::Item>;

    fn item_len(_: Self::Item) -> usize;
    fn items(self) -> Self::Iter;
    fn items_len(self) -> usize;
    fn split_at(self, _: usize) -> (Self, Self);
    /// Convert [Self::Iter] back into [Self].
    fn recover(_: Self::Iter) -> Self;
}
impl<'a> Parsable<'a> for &'a str {
    type Item = char;
    type Iter = str::Chars<'a>;

    fn item_len(ch: Self::Item) -> usize {
        ch.len_utf8()
    }
    fn items(self) -> Self::Iter {
        self.chars()
    }
    fn items_len(self) -> usize {
        self.len()
    }
    fn split_at(self, at: usize) -> (Self, Self) {
        self.split_at(at)
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

    fn item_len(_: Self::Item) -> usize {
        1
    }
    fn items(self) -> Self::Iter {
        self.iter()
    }
    fn items_len(self) -> usize {
        self.len()
    }
    fn split_at(self, at: usize) -> (Self, Self) {
        self.split_at(at)
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

    /// Filter the output of the current parser.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::{config::clisp::parser::{Parser, ParserOutput, ParserError, token::{Any, Just}}, either::Either};
    /// let is_a = Any::new().filter(|ch| 'a'.eq(ch), "is a");
    /// assert_eq!(is_a.parse("a"), Ok(ParserOutput::new("", 'a')));
    /// assert_eq!(is_a.parse("b"), Err(Either::Right(ParserError::Rule { item: 'b', rule: "is a" })));
    /// ```
    fn filter<F>(self, filter: F, rule: &'static str) -> Filter<'a, F, I, Self>
    where
        F: FnOnce(&Self::Output) -> bool
    {
        Filter {
            filter,
            parser: self,
            rule,
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
    fn flatten_err<E, O>(self) -> FlattenErr<Self>
    where
        Self: Parser<'a, I, Output = Result<O, E>>,
    {
        FlattenErr {
            parser: self,
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
        Map<F, Self>: Parser<'a, I, Error = Self::Error, Output = O>,
    {
        Map {
            parser: self,
            map,
        }
    }

    /// Transform the error of the current [Parser].
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::{config::clisp::parser::{Parser, ParserOutput, ParserError, token::Just}, either::Either};
    /// #[derive(Debug, PartialEq)]
    /// struct NotAError;
    /// let a = <Just<char> as Parser<'_, &str>>::map_err(Just('a'), |_: ParserError<char>| NotAError);
    /// assert_eq!(a.parse("a"), Ok(ParserOutput::new("", 'a')));
    /// assert_eq!(a.parse("b"), Err(NotAError));
    /// ```
    fn map_err<F, O>(self, map: F) -> MapErr<F, Self>
    where
        F: FnOnce(Self::Error) -> O,
        MapErr<F, Self>: Parser<'a, I, Error = O, Output = Self::Output>,
    {
        MapErr {
            map,
            parser: self,
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

/// Marker trait for [Parser]s where the output of the parser does not transform its input.
///
/// # Safety
///
/// - The returned length of [PureParser::output_len] must be accurate to its output.
/// - The returned length of [PureParser::output_len] must be safe to index into.
pub unsafe trait PureParser<'a, I>: Parser<'a, I>
where I: Parsable<'a> {
    /// Get the length of the current [Parser]'s output.
    fn output_len(_: Self::Output) -> usize;

    /// Return the current [Parser] as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::config::clisp::parser::{Parser, ParserOutput, PureParser, token::Just};
    /// let a = Just('a').restore();
    /// assert_eq!(a.parse("a"), Ok(ParserOutput::new("", "a")));
    /// ```
    fn restore(self) -> Restore<'a, I, Self> {
        Restore {
            parser: self,
            _marker: PhantomData,
        }
    }

    /// Repeat a [Parser] and return it as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::config::clisp::parser::{Parser, ParserOutput, PureParser, token::Just};
    /// let a_s = Just('a').repeated();
    /// assert_eq!(a_s.parse("aaabbb"), Ok(ParserOutput::new("bbb", "aaa")));
    /// ```
    fn repeated(self) -> Repeated<'a, I, Self>
    where Self: Clone {
        Repeated {
            parser: self,
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
impl Display for EofError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str("end of file")
    }
}
impl Error for EofError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParserError<T> {
    Eof(EofError),
    Match { expected: T, found: T },
    Rule {
        item: T,
        rule: &'static str,
    },
}
impl<T> ParserError<T> {
    pub fn map<F, O>(self, mut f: F) -> ParserError<O>
    where F: FnMut(T) -> O {
        match self {
            Self::Eof(e) => ParserError::Eof(e),
            Self::Match { expected, found } => ParserError::Match { expected: f(expected), found: f(found) },
            Self::Rule { item, rule } => ParserError::Rule { item: f(item), rule },
        }
    }
}
impl<T> Display for ParserError<T>
where
    T: Display
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Eof(e) => e.fmt(f),
            Self::Match { expected, found } => write!(f, "found `{found}` when expecting `{expected}`"),
            Self::Rule { item, rule } => write!(f, "`{item}` does not meet `{rule}`"),
        }
    }
}
impl<T> Error for ParserError<T>
where T: fmt::Debug + Display {}
impl<T> From<EofError> for ParserError<T> {
    fn from(err: EofError) -> Self {
        Self::Eof(err)
    }
}
