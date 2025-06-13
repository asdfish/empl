use crate::config::lisp::{
    parser::Parsable,
    parser2::{Parser, ParserInput},
};

#[derive(Clone, Copy, Debug)]
pub struct Any;
impl<'src, I> Parser<'src, I> for Any
where
    I: Parsable<'src>,
{
    type Output = I::Item;

    fn parse<'id>(&self, input: &mut ParserInput<'id, 'src, I>) -> Option<Self::Output> {
        input.write(|mut input| input.next())
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Just<T>(pub T)
where
    T: PartialEq;
impl<'src, I, T> Parser<'src, I> for Just<T>
where
    I: Parsable<'src, Item = T>,
    T: PartialEq,
{
    type Output = T;

    fn parse<'id>(&self, input: &mut ParserInput<'id, 'src, I>) -> Option<Self::Output> {
        input.branch(|mut input| input.next().filter(|item| self.0.eq(item)))
    }
}
