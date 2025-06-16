use {
    crate::config::lisp::{
        parser::Parsable,
        parser2::{Parser, ParserInput},
    },
    std::marker::PhantomData,
};

pub(super) trait ParserExt<'src, I>: Parser<'src, I>
where
    I: Parsable<'src>,
{
    fn iter<'id, 'input>(
        self,
        input: &'input mut ParserInput<'id, 'src, I>,
    ) -> Iter<'id, 'input, 'src, I, Self>
    where
        Self: Sized,
    {
        Iter {
            parser: self,
            input,
            _marker: PhantomData,
        }
    }
}
impl<'src, I, P> ParserExt<'src, I> for P
where
    I: Parsable<'src>,
    P: Parser<'src, I>,
{
}

pub struct Iter<'id, 'input, 'src, I, P>
where
    I: Parsable<'src>,
    P: Parser<'src, I>,
{
    pub(super) parser: P,
    pub(super) input: &'input mut ParserInput<'id, 'src, I>,
    pub(super) _marker: PhantomData<&'src ()>,
}
impl<'id, 'input, 'src, I, P> Iterator for Iter<'id, 'input, 'src, I, P>
where
    I: Parsable<'src>,
    P: Parser<'src, I>,
{
    type Item = P::Output;

    fn next(&mut self) -> Option<Self::Item> {
        self.parser.parse(self.input)
    }
}
