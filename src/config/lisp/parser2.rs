pub mod adapter;
pub mod token;

use {
    crate::config::lisp::{parser::Parsable, parser2::adapter::*},
    std::marker::PhantomData,
};

pub trait Parser<'a, I>
where
    I: Parsable<'a>,
{
    type Output;

    /// If the output is an [Err], it should be close to the error location.
    fn parse(
        &self,
        _: ParserInput<'a, I>,
    ) -> Result<(ParserInput<'a, I>, Self::Output), ParserInput<'a, I>>;

    fn filter<F>(self, predicate: F) -> Filter<'a, I, Self, F>
    where
        Self: Sized,
        F: Fn(&Self::Output) -> bool,
    {
        Filter {
            parser: self,
            predicate,
            _marker: PhantomData,
        }
    }

    fn filter_map<M, T>(self, morphism: M) -> impl Parser<'a, I, Output = T>
    where
        Self: Sized,
        M: Fn(Self::Output) -> Option<T>,
        I: 'a,
    {
        self.map(morphism).flatten()
    }

    fn flatten<T>(self) -> Flatten<'a, I, Self, T>
    where
        Self: Sized + Parser<'a, I, Output = Option<T>>,
    {
        Flatten {
            parser: self,
            _marker: PhantomData,
        }
    }

    fn map<M, T>(self, morphism: M) -> Map<'a, I, Self, M, T>
    where
        Self: Sized,
        M: Fn(Self::Output) -> T,
    {
        Map {
            parser: self,
            morphism,
            _marker: PhantomData,
        }
    }

    fn or<R>(self, r: R) -> Or<'a, I, Self, R>
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

    fn then<R>(self, r: R) -> Then<'a, I, Self, R>
    where
        Self: Sized,
        R: Parser<'a, I>,
    {
        Then {
            l: self,
            r,
            _marker: PhantomData,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ParserInput<'a, I>
where
    I: Parsable<'a>,
{
    contents: I,
    offset: usize,
    _marker: PhantomData<&'a ()>,
}
impl<'a, I> ParserInput<'a, I>
where
    I: Parsable<'a>,
{
    pub const fn new(contents: I) -> Self {
        Self {
            contents,
            offset: 0,
            _marker: PhantomData,
        }
    }
    pub const fn contents(&self) -> I {
        self.contents
    }
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// Get `n` elements as a slice.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::config::lisp::parser2::ParserInput;
    /// let mut input = ParserInput::new("goodbye");
    /// assert_eq!(input.get(4), Some("good"));
    /// assert_eq!(input.contents(), "bye");
    /// assert_eq!(input.offset(), 4);
    /// ```
    pub fn get(&mut self, n: usize) -> Option<I> {
        self.contents.split_at_checked(n).map(|(head, tail)| {
            self.contents = tail;
            self.offset += n;

            head
        })
    }
}
impl<'a, I> Iterator for ParserInput<'a, I>
where
    I: Parsable<'a>,
{
    type Item = I::Item;

    /// Get the next element in [Parsable::items], and update the offset.
    ///
    /// # Examples
    ///
    ///
    /// ```
    /// # use empl::config::lisp::parser2::ParserInput;
    /// [
    ///     ("ello", 1, Some('h')),
    ///     ("llo", 2, Some('e')),
    ///     ("lo", 3, Some('l')),
    ///     ("o", 4, Some('l')),
    ///     ("", 5, Some('o')),
    ///     ("", 5, None),
    /// ]
    /// .into_iter()
    /// .fold(
    ///     ParserInput::new("hello"),
    ///     |mut accum, (input, offset, next)| {
    ///         assert_eq!(accum.next(), next);
    ///         assert_eq!(accum.contents(), input);
    ///         assert_eq!(accum.offset(), offset);
    ///
    ///         accum
    ///     },
    /// );
    /// ```
    fn next(&mut self) -> Option<Self::Item> {
        let mut items = self.contents.items();
        items.next().inspect(|next| {
            self.offset += I::item_len(next);
            self.contents = I::recover(items);
        })
    }
}
