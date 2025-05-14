use {
    crate::{
        config::clisp::parser::{
            EofError, Parsable, Parser, ParserError, ParserOutput, PureParser,
        },
        either::EitherOrBoth,
    },
    std::marker::PhantomData,
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
    type Error = EofError;
    type Output = T;

    fn parse(self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        let mut items = input.items();
        let item = items.next().ok_or(EofError)?;

        Ok(ParserOutput::new(I::recover(items), item))
    }
}
// SAFETY: [Parsable::item_len] should be accurate
unsafe impl<'a, I, T> PureParser<'a, I> for Any<'a, I, T>
where
    I: Parsable<'a, Item = T>,
{
    fn output_len(output: Self::Output) -> usize {
        I::item_len(output)
    }
}

/// Identity parser that returns `self.0`
///
/// # Examples
///
/// ```
/// # use empl::config::clisp::parser::{Parser, ParserOutput, ParserError, token::Just};
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
    type Error = ParserError<T>;
    type Output = T;

    fn parse(self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        let mut items = input.items();

        match items.next().ok_or(ParserError::Eof(EofError))? {
            item if item == self.0 => Ok(ParserOutput::new(I::recover(items), item)),
            item => Err(ParserError::Match {
                expected: self.0,
                found: item,
            }),
        }
    }
}
unsafe impl<'a, I, T> PureParser<'a, I> for Just<T>
where
    I: Parsable<'a, Item = T>,
    T: PartialEq,
{
    fn output_len(output: Self::Output) -> usize {
        I::item_len(output)
    }
}

/// Identity parser for sequences
///
/// # Examples
/// ```
/// # use empl::config::clisp::parser::{Parser, ParserOutput, ParserError, token::Sequence};
/// assert_eq!(Sequence::new("hello").parse("hello world"), Ok(ParserOutput::new(" world", "hello")));
/// assert_eq!(Sequence::new("hello").parse("goodbye world"), Err(ParserError::Match { expected: 'h', found: 'g' }));
/// ```
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
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
    type Error = ParserError<I::Item>;
    type Output = I;

    fn parse(self, input: I) -> Result<ParserOutput<'a, I, Self::Output>, Self::Error> {
        let mut l = self.seq.items();
        let mut r = input.items();

        while let Some(state) = EitherOrBoth::new_lazy_left(|| l.next(), || r.next()) {
            match state {
                EitherOrBoth::Left(_) => return Err(ParserError::Eof(EofError)),
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
unsafe impl<'a, I> PureParser<'a, I> for Sequence<'a, I>
where
    I: Parsable<'a>,
    I::Item: PartialEq,
{
    fn output_len(output: Self::Output) -> usize {
        I::items_len(output)
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

            fn parse(self, input: Input) -> Result<ParserOutput<'a, Input, Self::Output>, Self::Error> {
                let Select(($($cdr,)* $car)) = self;

                $(if let Ok(output) = $cdr.parse(input) {
                    return Ok(output);
                })*

                $car.parse(input)
            }
        }

        // SAFETY: should be safe if all parsers are pure
        #[expect(non_camel_case_types)]
        unsafe impl<'a, Input, Output, $car, $($cdr),*> PureParser<'a, Input> for Select<($($cdr,)* $car)>
        where
            Input: Parsable<'a>,
            $car: Parser<'a, Input, Output = Output> + PureParser<'a, Input>,
            $($cdr: Parser<'a, Input, Output = Output> + PureParser<'a, Input>),*
        {
            fn output_len(output: Self::Output) -> usize {
                $car::output_len(output)
            }
        }

        impl_select!($($cdr),*);
    };
}
impl_select![a, b, c, d, e, f, g, h, i, j, k, l];
