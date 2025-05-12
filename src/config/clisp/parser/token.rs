use {
    crate::{
        config::clisp::parser::{Parsable, Parser, ParserError, ParserOutput},
        either::EitherOrBoth,
    },
    std::{convert::Infallible, marker::PhantomData},
};

#[derive(Clone, Copy, Debug, Default)]
#[repr(transparent)]
pub struct Any<'a, I, T>(PhantomData<&'a (I, T)>)
where
    I: Parsable<'a, Item = T>;
impl<'a, I, T> Any<'a, I, T>
where
    I: Parsable<'a, Item = T>,
{
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<'a, I, T> Parser<'a, I> for Any<'a, I, T>
where
    I: Parsable<'a, Item = T>,
{
    type Error = Infallible;
    type Output = T;

    fn parse(
        self,
        input: I,
    ) -> Result<ParserOutput<'a, I, Self::Output>, ParserError<I::Item, Self::Error>> {
        let mut items = input.items();
        let item = items.next().ok_or(ParserError::Eof)?;

        Ok(ParserOutput::new(I::recover(items), item))
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
    I: Parsable<'a, Item = T>,
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
    T: Parsable<'a>,
    T::Item: PartialEq,
{
    seq: T,
    _marker: PhantomData<&'a ()>,
}
impl<'a, T> Sequence<'a, T>
where
    T: Parsable<'a>,
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
    I: Parsable<'a>,
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
                    });
                }
            }
        }

        Ok(ParserOutput::new(I::recover(r), self.seq))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Select<T>(pub T);
macro_rules! impl_select {
    ($car:ident) => {};
    ($car:ident, $($cdr:ident),* $(,)?) => {
        #[expect(non_camel_case_types)]
        impl<'a, Input, Output, $car, $($cdr),*> Parser<'a, Input> for Select<($($cdr,)* $car)>
        where
            Input: Parsable<'a>,
            $car: Parser<'a, Input, Output = Output>,
            $($cdr: Parser<'a, Input, Output = Output>),*
        {
            type Error = $car::Error;
            type Output = Output;

            fn parse(self, input: Input) -> Result<ParserOutput<'a, Input, Self::Output>, ParserError<Input::Item, Self::Error>> {
                let Select(($($cdr,)* $car)) = self;

                $(if let Ok(output) = $cdr.parse(input) {
                    return Ok(output);
                })*

                $car.parse(input)
            }
        }

        impl_select!($($cdr),*);
    };
}
impl_select![a, b, c, d, e, f, g, h, i, j, k, l];
