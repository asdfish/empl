use crate::ext::command::{CommandChain, CommandIter};

pub trait IteratorExt: Iterator + Sized {
    fn map_command<M, C>(self, map: M) -> CommandIter<Self, Self::Item, M, C>
    where
        M: FnMut(Self::Item) -> C,
        C: CommandChain,
    {
        CommandIter(self, map)
    }
}
impl<T> IteratorExt for T
where T: Iterator {}
