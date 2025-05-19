use std::borrow::Borrow;

#[derive(Debug, PartialEq)]
pub enum NonStaticCow<'a, T>
where
    T: ToOwned + ?Sized,
{
    Borrowed(&'a T),
    Owned(T::Owned),
}
impl<'a, T> Clone for NonStaticCow<'a, T>
where
    T: ToOwned + ?Sized,
    T::Owned: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Borrowed(b) => Self::Borrowed(b),
            Self::Owned(t) => Self::Owned(t.clone()),
        }
    }
}
impl<'a, T> NonStaticCow<'a, T>
where
    T: ToOwned + ?Sized,
{
    pub fn into_owned(self) -> T::Owned {
        match self {
            Self::Borrowed(b) => b.to_owned(),
            Self::Owned(b) => b,
        }
    }
}
impl<'a, T> AsRef<T> for NonStaticCow<'a, T>
where
    T: ToOwned + ?Sized,
{
    fn as_ref(&self) -> &T {
        match self {
            Self::Borrowed(t) => t,
            Self::Owned(t) => t.borrow(),
        }
    }
}
