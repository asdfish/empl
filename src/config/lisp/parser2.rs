pub mod adapter;
pub mod token;

use {
    crate::config::lisp::{parser::Parsable, parser2::adapter::*},
    generativity::{make_guard, Guard},
    std::marker::PhantomData,
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
    pub const fn new(input: I, _guard: Guard<'id>) -> Self {
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

    pub fn branch<F, T>(&mut self, branch: F) -> Option<T>
    where
        F: for<'input> FnOnce(ParserInputMut<'id, 'input, 'src, I>) -> Option<T>,
    {
        let mut copy_inner = self.clone();
        let copy = ParserInputMut(&mut copy_inner);
        branch(copy).inspect(|_| *self = copy_inner)
    }
    pub fn write<'input, F, T>(&'input mut self, operation: F) -> T
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

    fn or<R>(self, r: R) -> Or<'src, I, Self, R>
    where
        Self: Sized,
        R: Parser<'src, I, Output = Self::Output>,
    {
        Or {
            l: self,
            r,
            _marker: PhantomData,
        }
    }
}
