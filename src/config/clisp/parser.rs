//! Parser combinators

pub mod adapter;
pub mod token;

use {
    crate::config::clisp::parser::adapter::*,
    std::{
        error::Error,
        fmt::{self, Display, Formatter},
        marker::PhantomData,
        slice, str,
    },
};

/// Trait for types that can be used by a [Parser].
pub trait Parsable<'a>: Copy + Sized {
    type Item: Copy + 'a;
    type Iter: Iterator<Item = Self::Item>;

    fn item_len(_: Self::Item) -> usize;
    fn items(self) -> Self::Iter;
    fn items_len(self) -> usize;
    fn split_at(self, _: usize) -> (Self, Self);
    fn split_at_checked(self, _: usize) -> Option<(Self, Self)>;
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
    fn split_at_checked(self, at: usize) -> Option<(Self, Self)> {
        self.split_at_checked(at)
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
    fn split_at_checked(self, at: usize) -> Option<(Self, Self)> {
        self.split_at_checked(at)
    }
    fn recover(items: Self::Iter) -> &'a [T] {
        items.as_slice()
    }
}

pub trait Parser<'a, I>
where
    I: Parsable<'a>,
{
    type Error;
    type Output;

    /// The parser next part of the parser output's length must be smaller or equal to the input's.
    fn parse(&self, _: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error>;

    /// Get the error of the parser as a result, so that you can use it to recover.
    fn co_flatten_err(self) -> CoFlattenErr<'a, I, Self>
    where
        Self: Sized,
    {
        CoFlattenErr {
            parser: self,
            _marker: PhantomData,
        }
    }

    fn delimited_by<E, L, R>(self, l: L, r: R) -> DelimitedBy<'a, I, E, L, Self, R>
    where
        Self: Parser<'a, I, Error = E> + Sized,
        L: Parser<'a, I, Error = E>,
        R: Parser<'a, I, Error = E>,
    {
        DelimitedBy {
            l,
            parser: self,
            r,
            _marker: PhantomData,
        }
    }

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
        Self: Sized,
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
    /// # use empl::config::clisp::parser::{Parser, ParserOutput, token::Any};
    /// #[derive(Debug, PartialEq)]
    /// struct NotAError;
    /// let is_a = Any::new().map_err(|_| NotAError).filter(|_| NotAError, |ch| 'a'.eq(ch));
    /// assert_eq!(is_a.parse("a"), Ok(ParserOutput::new("", 'a')));
    /// assert_eq!(is_a.parse("b"), Err(NotAError));
    /// ```
    fn filter<E, F>(self, error: E, predicate: F) -> Filter<'a, E, F, I, Self>
    where
        Self: Sized,
        E: Fn(Self::Output) -> Self::Error,
        F: Fn(&Self::Output) -> bool,
    {
        Filter {
            error,
            parser: self,
            predicate,
            _marker: PhantomData,
        }
    }

    /// Filters the current output while allowing mapping for more type safe filtering.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::config::clisp::parser::{Parser, ParserOutput, token::Any};
    /// #[derive(Debug, PartialEq)]
    /// struct NonDigitError;
    /// let digit = Any::new().map_err(|_| NonDigitError).filter_map(|ch: char| ch.to_digit(10).ok_or(NonDigitError));
    /// assert_eq!(digit.parse("1"), Ok(ParserOutput::new("", 1)));
    /// assert_eq!(digit.parse("a"), Err(NonDigitError));
    /// ```
    fn filter_map<M, T>(self, map: M) -> FilterMap<'a, Self::Error, I, M, Self, T>
    where
        Self: Sized,
        M: Fn(Self::Output) -> Result<T, Self::Error>,
    {
        FilterMap {
            map,
            parser: self,
            _marker: PhantomData,
        }
    }

    /// Convert a `Result<T, E>` to a `T` by making the error part of the parser.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::config::clisp::parser::{Parser, ParserOutput, ParserError, token::Any};
    /// #[derive(Debug, PartialEq)]
    /// struct MyError;
    /// let end = Any::new().map_err(|_| MyError).map(|_| Err::<(), MyError>(MyError)).flatten_err();
    /// assert_eq!(end.parse("asdf"), Err(MyError));
    /// ```
    fn flatten_err<E, O>(self) -> FlattenErr<'a, I, E, O, Self>
    where
        Self: Parser<'a, I, Error = E, Output = Result<O, E>> + Sized,
    {
        FlattenErr {
            parser: self,
            _marker: PhantomData,
        }
    }

    /// Accumulate the output of this parser as well as the previous parts as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::config::clisp::parser::{Parser, ParserOutput, token::Just};
    /// # use std::convert::Infallible;
    /// let a_count = Just('a').fold(|| 0, |accum, _, _| Ok::<usize, Infallible>(accum + 1));
    /// assert_eq!(a_count.parse("aaa"), Ok(ParserOutput::new("", 3)));
    /// let a_count = Just('a').fold(|| 0, |_, slice: &str, _| Ok::<usize, Infallible>(slice.len()));
    /// assert_eq!(a_count.parse("aaa"), Ok(ParserOutput::new("", 3)));
    /// ```
    fn fold<A, AF, E, F>(self, start: AF, fold: F) -> Fold<'a, A, AF, E, F, I, Self>
    where
        Self: Clone + Sized,
        AF: Fn() -> A,
        F: Fn(A, I, Self::Output) -> Result<A, E>,
    {
        Fold {
            fold,
            parser: self,
            start,
            _marker: PhantomData,
        }
    }

    fn ignore_then<R>(self, r: R) -> IgnoreThen<'a, I, Self::Error, Self, R>
    where
        Self: Sized,
        R: Parser<'a, I, Error = Self::Error>,
    {
        IgnoreThen {
            l: self,
            r,
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
    fn map<F, O>(self, map: F) -> Map<'a, I, F, O, Self>
    where
        Self: Sized,
        F: Fn(Self::Output) -> O,
    {
        Map {
            parser: self,
            map,
            _marker: PhantomData,
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
    fn map_err<F, O>(self, map: F) -> MapErr<'a, I, F, O, Self>
    where
        Self: Sized,
        F: Fn(Self::Error) -> O,
    {
        MapErr {
            map,
            parser: self,
            _marker: PhantomData,
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
        Self: Sized,
        F: Fn(&mut Iter<'a, I, &Self>) -> O,
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
        Self: Sized,
        R: Parser<'a, I, Output = Self::Output>,
    {
        Or {
            l: self,
            r,
            _marker: PhantomData,
        }
    }

    /// Chain two parsers together with an output of `(Self::Output, R::Output)`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::config::clisp::parser::{Parser, ParserOutput, ParserError, token::Just};
    /// assert_eq!(Just('h').then(Just('i')).parse("hi"), Ok(ParserOutput::new("", ('h', 'i'))));
    /// assert_eq!(Just('h').then(Just('i')).parse("ho"), Err(ParserError::Match { expected: 'i', found: 'o' }));
    /// ```
    fn then<R>(self, r: R) -> Then<'a, I, Self::Error, Self, R>
    where
        Self: Sized,
        R: Parser<'a, I, Error = Self::Error>,
    {
        Then {
            l: self,
            r,
            _marker: PhantomData,
        }
    }

    fn then_ignore<R>(self, r: R) -> ThenIgnore<'a, I, Self::Error, Self, R>
    where
        Self: Sized,
        R: Parser<'a, I, Error = Self::Error>,
    {
        ThenIgnore {
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
    /// let is_a = Just('a').to(|| true).or(Any::new().to(|| false));
    /// assert_eq!(is_a.parse("a"), Ok(ParserOutput::new("", true)));
    /// assert_eq!(is_a.parse("b"), Ok(ParserOutput::new("", false)));
    /// ```
    fn to<T, TF>(self, to: TF) -> To<'a, I, Self, T, TF>
    where
        Self: Sized,
        TF: Fn() -> T,
    {
        To {
            parser: self,
            to,
            _marker: PhantomData,
        }
    }
}
impl<'a, I, T> Parser<'a, I> for &T
where
    I: Parsable<'a>,
    T: Parser<'a, I>,
{
    type Error = T::Error;
    type Output = T::Output;

    fn parse(&self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        (*self).parse(input)
    }
}

/// Marker trait for [Parser]s where the output of the parser does not transform its input.
///
/// # Safety
///
/// - The returned length of [PureParser::output_len] must be accurate to its output.
/// - The returned length of [PureParser::output_len] must be safe to index into.
pub unsafe trait PureParser<'a, I>: Parser<'a, I>
where
    I: Parsable<'a>,
{
    /// Get the length of the current [Parser]'s output.
    fn output_len(_: Self::Output) -> usize;

    /// Return the current [Parser] as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::config::clisp::parser::{Parser, ParserOutput, PureParser, token::Just};
    /// let a = Just('a').as_slice();
    /// assert_eq!(a.parse("a"), Ok(ParserOutput::new("", "a")));
    /// ```
    #[expect(clippy::wrong_self_convention)]
    fn as_slice(self) -> AsSlice<'a, I, Self>
    where
        Self: Sized,
    {
        AsSlice {
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
    where
        Self: Clone,
    {
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

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EofError;
impl Display for EofError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str("end of file")
    }
}
impl Error for EofError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParserError<T>
where
    T: ToOwned,
{
    Eof(EofError),
    Match { expected: T, found: T },
}
impl<T> Display for ParserError<T>
where
    T: Display + ToOwned,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Eof(e) => e.fmt(f),
            Self::Match { expected, found } => {
                write!(f, "`{found}` does not match `{expected}`")
            }
        }
    }
}
impl<T> Error for ParserError<T> where T: fmt::Debug + Display + ToOwned {}
