//! Parser combinators

pub mod adapter;
pub mod recursive;
pub mod token;

use {
    crate::{config::clisp::parser::adapter::*, either::Either},
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
    type Output;

    /// The parser next part of the parser output's length must be smaller or equal to the input's.
    fn parse(&self, _: I) -> Option<ParserOutput<'a, I, Self::Output>>;

    /// Get the error of the parser as a result, so that you can use it to recover.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::config::clisp::{lexer::IntParser, parser::{Parser, ParserOutput}};
    /// let int_or_0 = IntParser::<10, u32>::new().co_flatten().map(|output| output.unwrap_or(0));
    /// assert_eq!(int_or_0.parse("10"), Some(ParserOutput::new("", 10)));
    /// assert_eq!(int_or_0.parse("foo"), Some(ParserOutput::new("foo", 0)));
    /// ```
    fn co_flatten(self) -> CoFlatten<'a, I, Self>
    where
        Self: Sized,
        CoFlatten<'a, I, Self>:
            Parser<'a, I, Output = Option<Self::Output>>,
    {
        CoFlatten {
            parser: self,
            _marker: PhantomData,
        }
    }

    /// Surround the parser and ignore them.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::config::clisp::{lexer::IdentParser, parser::{Parser, ParserOutput, token::Just}};
    /// let string_interpolation = IdentParser.delimited_by(Just('{'), Just('}'));
    /// assert_eq!(string_interpolation.parse("{foo}"), Some(ParserOutput::new("", "foo")));
    /// ```
    fn delimited_by<L, R>(self, l: L, r: R) -> DelimitedBy<'a, I, L, Self, R>
    where
        Self: Parser<'a, I> + Sized,
        L: Parser<'a, I>,
        R: Parser<'a, I>,
        DelimitedBy<'a, I, L, Self, R>: Parser<'a, I, Output = Self::Output>,
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
    /// # use empl::{config::clisp::parser::{Parser, ParserOutput, token::{Just, Sequence}}, either::Either};
    /// let abc = Just('a').either_or(Sequence::new("bc"));
    /// assert_eq!(abc.parse("a"), Some(ParserOutput::new("", Either::Left('a'))));
    /// assert_eq!(abc.parse("bc"), Some(ParserOutput::new("", Either::Right("bc"))));
    /// ```
    fn either_or<R>(self, r: R) -> EitherOr<'a, I, Self, R>
    where
        Self: Sized,
        R: Parser<'a, I>,
        EitherOr<'a, I, Self, R>:
            Parser<'a, I, Output = Either<Self::Output, R::Output>>,
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
    /// let is_a = Any::new().filter(|ch: &char| 'a'.eq(ch));
    /// assert_eq!(is_a.parse("a"), Some(ParserOutput::new("", 'a')));
    /// assert_eq!(is_a.parse("b"), None);
    /// ```
    fn filter<F>(self, predicate: F) -> Filter<'a, F, I, Self>
    where
        Self: Sized,
        F: Fn(&Self::Output) -> bool,
        Filter<'a, F, I, Self>: Parser<'a, I, Output = Self::Output>,
    {
        Filter {
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
    /// let digit = Any::new().filter_map(|ch: char| ch.to_digit(10));
    /// assert_eq!(digit.parse("1"), Some(ParserOutput::new("", 1)));
    /// assert_eq!(digit.parse("a"), None);
    /// ```
    fn filter_map<M, T>(self, map: M) -> FilterMap<'a, I, M, Self, T>
    where
        Self: Sized,
        M: Fn(Self::Output) -> Option<T>,
        FilterMap<'a, I, M, Self, T>: Parser<'a, I, Output = T>,
    {
        FilterMap {
            map,
            parser: self,
            _marker: PhantomData,
        }
    }

    fn flatten<T>(self) -> Flatten<'a, I, Self, T>
    where Self: Parser<'a, I, Output = Option<T>> + Sized {
        Flatten {
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
    /// let a_count = Just('a').fold(|| 0, |accum, _, _| Some(accum + 1));
    /// assert_eq!(a_count.parse("aaa"), Some(ParserOutput::new("", 3)));
    /// let a_count = Just('a').fold(|| 0, |_, slice: &str, _| Some(slice.len()));
    /// assert_eq!(a_count.parse("aaa"), Some(ParserOutput::new("", 3)));
    /// ```
    fn fold<A, AF, F>(self, start: AF, fold: F) -> Fold<'a, A, AF, F, I, Self>
    where
        Self: Clone + Sized,
        AF: Fn() -> A,
        F: Fn(A, I, Self::Output) -> Option<A>,
        Fold<'a, A, AF, F, I, Self>: Parser<'a, I, Output = A>,
    {
        Fold {
            fold,
            parser: self,
            start,
            _marker: PhantomData,
        }
    }

    /// Ignore the output of the current parser and use the next parser.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::config::clisp::parser::{Parser, ParserOutput, token::Just};
    /// let ab = Just('a').ignore_then(Just('b'));
    /// assert_eq!(ab.parse("ab"), Some(ParserOutput::new("", 'b')));
    /// ```
    fn ignore_then<R>(self, r: R) -> IgnoreThen<'a, I, Self, R>
    where
        Self: Sized,
        R: Parser<'a, I>,
        IgnoreThen<'a, I, Self, R>:
            Parser<'a, I, Output = R::Output>,
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
    /// assert_eq!(lowercase.parse("a"), Some(ParserOutput::new("", 'a')));
    /// assert_eq!(lowercase.parse("A"), Some(ParserOutput::new("", 'a')));
    /// ```
    fn map<F, O>(self, map: F) -> Map<'a, I, F, O, Self>
    where
        Self: Sized,
        F: Fn(Self::Output) -> O,
        Map<'a, I, F, O, Self>: Parser<'a, I, Output = O>,
    {
        Map {
            parser: self,
            map,
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
    /// assert_eq!(count_a.parse("aaa"), Some(ParserOutput::new("", 3)));
    /// assert_eq!(count_a.parse("aaabbb"), Some(ParserOutput::new("bbb", 3)));
    /// ```
    fn map_iter<F, O>(self, map: F) -> MapIter<'a, I, F, O, Self>
    where
        Self: Sized,
        F: Fn(&mut Iter<'a, I, &Self>) -> O,
        MapIter<'a, I, F, O, Self>: Parser<'a, I, Output = O>,
    {
        MapIter {
            parser: self,
            map,
            _marker: PhantomData,
        }
    }

    /// Make the output of this parser optional.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::config::clisp::parser::{Parser, ParserOutput, token::Just};
    /// let maybe_a = Just('a').maybe();
    /// assert_eq!(maybe_a.parse("a"), Some(ParserOutput::new("", Some('a'))));
    /// assert_eq!(maybe_a.parse("b"), Some(ParserOutput::new("b", None)));
    /// ```
    fn maybe(self) -> Maybe<'a, I, Self>
    where
        Self: Sized,
        Maybe<'a, I, Self>: Parser<'a, I, Output = Option<Self::Output>>,
    {
        Maybe {
            parser: self,
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
    /// assert_eq!(a_or_b.parse("a"), Some(ParserOutput::new("", 'a')));
    /// assert_eq!(a_or_b.parse("b"), Some(ParserOutput::new("", 'b')));
    /// assert_eq!(a_or_b.parse("c"), None);
    /// ```
    fn or<R>(self, r: R) -> Or<'a, I, Self::Output, Self, R>
    where
        Self: Sized,
        R: Parser<'a, I, Output = Self::Output>,
        Or<'a, I, Self::Output, Self, R>: Parser<'a, I, Output = Self::Output>,
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
    /// assert_eq!(Just('h').then(Just('i')).parse("hi"), Some(ParserOutput::new("", ('h', 'i'))));
    /// assert_eq!(Just('h').then(Just('i')).parse("ho"), None);
    /// ```
    fn then<R>(self, r: R) -> Then<'a, I, Self, R>
    where
        Self: Sized,
        R: Parser<'a, I>,
        Then<'a, I, Self, R>:
            Parser<'a, I, Output = (Self::Output, R::Output)>,
    {
        Then {
            l: self,
            r,
            _marker: PhantomData,
        }
    }

    /// Ignore the output of the next parser and keep current one's output.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::config::clisp::parser::{Parser, ParserOutput, token::Just};
    /// let ab = Just('a').then_ignore(Just('b'));
    /// assert_eq!(ab.parse("ab"), Some(ParserOutput::new("", 'a')));
    /// ```
    fn then_ignore<R>(self, r: R) -> ThenIgnore<'a, I, Self, R>
    where
        Self: Sized,
        R: Parser<'a, I>,
        ThenIgnore<'a, I, Self, R>:
            Parser<'a, I, Output = Self::Output>,
    {
        ThenIgnore {
            l: self,
            r,
            _marker: PhantomData,
        }
    }
}
impl<'a, I, T> Parser<'a, I> for &T
where
    I: Parsable<'a>,
    T: Parser<'a, I> + ?Sized,
{
    type Output = T::Output;

    fn parse(&self, input: I) -> Option<ParserOutput<'a, I, Self::Output>> {
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
    /// assert_eq!(a.parse("a"), Some(ParserOutput::new("", "a")));
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
    /// assert_eq!(a_s.parse("aaabbb"), Some(ParserOutput::new("bbb", "aaa")));
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
impl<'a, I, T> ParserOutput<'a, I, Option<T>>
where
    I: Parsable<'a>,
{
    pub fn transpose(self) -> Option<ParserOutput<'a, I, T>> {
        match self.output {
            Some(output) => Some(ParserOutput {
                next: self.next,
                output,
                _marker: PhantomData,
            }),
            _ => None,
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
