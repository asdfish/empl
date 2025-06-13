use crate::config::lisp::{
    parser::Parsable,
    parser2::{Parser, ParserInput},
};

pub struct Any;
impl<'a, I> Parser<'a, I> for Any
where
    I: Parsable<'a>,
{
    type Output = I::Item;

    fn parse(
        &self,
        mut input: ParserInput<'a, I>,
    ) -> Result<(ParserInput<'a, I>, Self::Output), ParserInput<'a, I>> {
        input.next().map(|output| (input, output)).ok_or(input)
    }
}

pub struct Empty;
impl<'a, I> Parser<'a, I> for Empty
where
    I: Parsable<'a>,
{
    type Output = ();

    fn parse(
        &self,
        input: ParserInput<'a, I>,
    ) -> Result<(ParserInput<'a, I>, Self::Output), ParserInput<'a, I>> {
        Ok((input, ()))
    }
}

pub struct Just<T>(T);
impl<'a, I, T> Parser<'a, I> for Just<T>
where
    I: Parsable<'a, Item = T>,
    T: PartialEq,
{
    type Output = T;

    fn parse(
        &self,
        mut input: ParserInput<'a, I>,
    ) -> Result<(ParserInput<'a, I>, Self::Output), ParserInput<'a, I>> {
        input
            .next()
            .filter(|item| item.eq(&self.0))
            .map(move |item| (input, item))
            .ok_or(input)
    }
}
