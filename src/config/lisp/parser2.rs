pub mod adapter;
pub mod token;

use {
    crate::{
        config::lisp::{
            parser::Parsable,
            parser2::adapter::{Iter, ParserExt},
        },
        either::Either,
        ext::pair::BiFunctor,
    },
    generativity::{make_guard, Guard},
    std::{marker::PhantomData, ops::Deref},
};

pub fn with_scope<'src, I, O, T>(input: I, operation: O) -> T
where
    I: Parsable<'src>,
    O: for<'id> FnOnce(ParserInput<'id, 'src, I>) -> T,
{
    make_guard!(guard);
    operation(ParserInput::new(input, guard))
}

#[derive(Debug)]
pub struct ParserInput<'id, 'src, I>
where
    I: Parsable<'src>,
{
    contents: I,
    offset: usize,
    _marker: PhantomData<&'id &'src ()>,
}
impl<'id, 'src, I> ParserInput<'id, 'src, I>
where
    I: Parsable<'src>,
{
    const fn offset(&self) -> usize {
        self.offset
    }

    const fn new(input: I, _guard: Guard<'id>) -> Self {
        Self {
            contents: input,
            offset: 0,
            _marker: PhantomData,
        }
    }
    fn clone(&self) -> Self {
        Self {
            contents: self.contents,
            offset: self.offset,
            _marker: PhantomData,
        }
    }

    fn branch<F, T>(&mut self, branch: F) -> Option<T>
    where
        F: for<'input> FnOnce(ParserInputMut<'id, 'input, 'src, I>) -> Option<T>,
    {
        let mut copy_inner = self.clone();
        let copy = ParserInputMut(&mut copy_inner);
        branch(copy).inspect(|_| *self = copy_inner)
    }
    fn write<'input, F, T>(&'input mut self, operation: F) -> T
    where
        F: FnOnce(ParserInputMut<'id, 'input, 'src, I>) -> T,
    {
        operation(ParserInputMut(self))
    }
}

#[repr(transparent)]
pub struct ParserInputMut<'id, 'input, 'src, I>(&'input mut ParserInput<'id, 'src, I>)
where
    I: Parsable<'src>;
impl<'id, 'input, 'src, I> AsMut<ParserInput<'id, 'src, I>> for ParserInputMut<'id, 'input, 'src, I>
where
    I: Parsable<'src>,
{
    fn as_mut(&mut self) -> &mut ParserInput<'id, 'src, I> {
        &mut self.0
    }
}
impl<'id, 'input, 'src, I> Deref for ParserInputMut<'id, 'input, 'src, I>
where
    I: Parsable<'src>,
{
    type Target = ParserInput<'id, 'src, I>;

    fn deref(&self) -> &ParserInput<'id, 'src, I> {
        &self.0
    }
}
impl<'id, 'input, 'src, I> Iterator for ParserInputMut<'id, 'input, 'src, I>
where
    I: Parsable<'src>,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let mut items = self.0.contents.items();
        items.next().inspect(move |item| {
            self.0.offset += I::item_len(item);
            self.0.contents = I::recover(items);
        })
    }
}

pub trait Parser<'src, I>
where
    I: Parsable<'src>,
{
    type Output;

    fn parse<'id>(&self, input: &mut ParserInput<'id, 'src, I>) -> Option<Self::Output>;

    fn as_slice(self) -> impl Parser<'src, I, Output = I>
    where
        Self: Sized,
    {
        parser_fn(move |input| {
            input.branch(|mut input| {
                let ParserInput {
                    contents, offset, ..
                } = *input.deref();

                self.parse(input.as_mut())
                    .and_then(|_| input.offset().checked_sub(offset))
                    .and_then(|length| contents.split_at_checked(length))
                    .map(<(_, _)>::fst)
            })
        })
    }

    fn co_flatten(self) -> impl Parser<'src, I, Output = Option<Self::Output>>
    where
        Self: Sized,
    {
        parser_fn(move |input| Some(self.parse(input)))
    }

    fn delimited_by<L, R>(self, l: L, r: R) -> impl Parser<'src, I, Output = Self::Output>
    where
        Self: Sized,
        L: Parser<'src, I>,
        R: Parser<'src, I>,
    {
        l.ignore_then(self).then_ignore(r)
    }

    fn either_or<R>(self, r: R) -> impl Parser<'src, I, Output = Either<Self::Output, R::Output>>
    where
        Self: Sized,
        R: Parser<'src, I>,
    {
        parser_fn(move |input| {
            input
                .branch(|mut input| self.parse(input.as_mut()).map(Either::Left))
                .or_else(|| input.branch(|mut input| r.parse(input.as_mut()).map(Either::Right)))
        })
    }

    fn filter<F>(self, predicate: F) -> impl Parser<'src, I, Output = Self::Output>
    where
        Self: Sized,
        F: Fn(&Self::Output) -> bool,
    {
        parser_fn(move |input| {
            input.branch(|mut input| self.parse(input.as_mut()).filter(&predicate))
        })
    }

    fn filter_map<F, T>(self, morphism: F) -> impl Parser<'src, I, Output = T>
    where
        Self: Sized,
        F: Fn(Self::Output) -> Option<T>,
    {
        self.map(morphism).flatten()
    }

    fn flatten<T>(self) -> impl Parser<'src, I, Output = T>
    where
        Self: Parser<'src, I, Output = Option<T>> + Sized,
    {
        parser_fn(move |input| input.branch(|mut input| self.parse(input.as_mut()).flatten()))
    }

    fn ignore_then<R>(self, r: R) -> impl Parser<'src, I, Output = R::Output>
    where
        Self: Sized,
        R: Parser<'src, I>,
    {
        parser_fn(move |input| {
            input.branch(|mut input| {
                self.parse(input.as_mut())
                    .and_then(|_| r.parse(input.as_mut()))
            })
        })
    }

    fn map<M, T>(self, morphism: M) -> impl Parser<'src, I, Output = T>
    where
        Self: Sized,
        M: Fn(Self::Output) -> T,
    {
        parser_fn(move |input| self.parse(input).map(&morphism))
    }

    fn map_iter<M, T>(self, morphism: M) -> impl Parser<'src, I, Output = T>
    where
        Self: Sized,
        M: for<'id, 'input> Fn(Iter<'id, 'input, 'src, I, &Self>) -> Option<T>,
    {
        parser_fn(move |input| (morphism)((&self).iter(input)))
    }

    /// Get either parser.
    ///
    /// # Examples
    /// ```
    /// use empl::config::lisp::parser2::{token::Just, with_scope, Parser, ParserInput};
    /// let a_or_b = Just('a').or(Just('b'));
    /// with_scope("a", |mut input| {
    ///     assert_eq!(a_or_b.parse(&mut input), Some('a'))
    /// });
    /// with_scope("b", |mut input| {
    ///     assert_eq!(a_or_b.parse(&mut input), Some('b'))
    /// });
    /// with_scope("c", |mut input| assert_eq!(a_or_b.parse(&mut input), None));
    /// ```
    fn or<R>(self, r: R) -> impl Parser<'src, I, Output = Self::Output>
    where
        Self: Sized,
        R: Parser<'src, I, Output = Self::Output>,
    {
        parser_fn(move |input| {
            input
                .branch(|mut input| self.parse(input.as_mut()))
                .or_else(|| input.branch(|mut input| r.parse(input.as_mut())))
        })
    }

    fn repeated(self) -> impl Parser<'src, I, Output = ()>
    where
        Self: Sized,
    {
        parser_fn(move |input| (&self).iter(input).last().map(drop))
    }

    fn then<R>(self, r: R) -> impl Parser<'src, I, Output = (Self::Output, R::Output)>
    where
        Self: Sized,
        R: Parser<'src, I>,
    {
        parser_fn(move |input| {
            input.branch(|mut input| {
                self.parse(input.as_mut())
                    .and_then(|l| r.parse(input.as_mut()).map(move |r| (l, r)))
            })
        })
    }

    fn then_ignore<R>(self, r: R) -> impl Parser<'src, I, Output = Self::Output>
    where
        Self: Sized,
        R: Parser<'src, I>,
    {
        parser_fn(move |input| {
            input.branch(|mut input| {
                self.parse(input.as_mut())
                    .and_then(|output| r.parse(input.as_mut()).map(move |_| output))
            })
        })
    }
}
impl<'src, I, P> Parser<'src, I> for &P
where
    I: Parsable<'src>,
    P: Parser<'src, I>,
{
    type Output = P::Output;

    fn parse<'id>(&self, input: &mut ParserInput<'id, 'src, I>) -> Option<Self::Output> {
        <P as Parser<'src, I>>::parse(*self, input)
    }
}

pub fn parser_fn<'src, I, F, T>(parser: F) -> ParserFn<'src, I, F, T>
where
    I: Parsable<'src>,
    F: for<'id> Fn(&mut ParserInput<'id, 'src, I>) -> Option<T>,
{
    ParserFn {
        parser,
        _marker: PhantomData,
    }
}

pub struct ParserFn<'src, I, F, T>
where
    I: Parsable<'src>,
    F: for<'id> Fn(&mut ParserInput<'id, 'src, I>) -> Option<T>,
{
    parser: F,
    _marker: PhantomData<&'src I>,
}
impl<'src, I, F, T> Parser<'src, I> for ParserFn<'src, I, F, T>
where
    I: Parsable<'src>,
    F: for<'id> Fn(&mut ParserInput<'id, 'src, I>) -> Option<T>,
{
    type Output = T;

    fn parse<'id>(&self, input: &mut ParserInput<'id, 'src, I>) -> Option<Self::Output> {
        (self.parser)(input)
    }
}
