use nonempty_collections::iter::NonEmptyIterator;

pub trait NonEmptyIteratorExt: NonEmptyIterator {
    fn last(self) -> Self::Item
    where Self: Sized {
        self.reduce(|_, i| i)
    }
}
impl<T> NonEmptyIteratorExt for T
where T: NonEmptyIterator {}
