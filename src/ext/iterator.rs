use crate::ext::command::{CommandChain, CommandIter};

pub trait IteratorExt<T>: Iterator<Item = T> + Sized
where
    T: CommandChain,
{
    fn adapt(self) -> CommandIter<Self, T> {
        CommandIter(self)
    }
}
impl<I, T> IteratorExt<T> for I
where
    I: Iterator<Item = T>,
    T: CommandChain,
{
}
