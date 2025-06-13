use {
    crate::config::lisp::{
        parser::Parsable,
        parser2::{Parser, ParserInput},
    },
    std::marker::PhantomData,
};

pub struct Or<'a, I, L, R>
where
    I: Parsable<'a>,
    L: Parser<'a, I>,
    R: Parser<'a, I, Output = L::Output>,
{
    pub(super) l: L,
    pub(super) r: R,
    pub(super) _marker: PhantomData<&'a I>,
}
impl<'src, I, L, R> Parser<'src, I> for Or<'src, I, L, R>
where
    I: Parsable<'src>,
    L: Parser<'src, I>,
    R: Parser<'src, I, Output = L::Output>,
{
    type Output = L::Output;

    fn parse<'id>(&self, input: &mut ParserInput<'id, 'src, I>) -> Option<Self::Output> {
        input
            .branch(|mut input| self.l.parse(input.as_mut()))
            .or_else(|| input.branch(|mut input| self.r.parse(input.as_mut())))
    }
}
