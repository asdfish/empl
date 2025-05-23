use std::{borrow::Cow, ops::Deref, rc::Rc};

#[derive(Debug)]
pub enum LazyRc<'a, T>
where
    T: ?Sized,
{
    Borrowed(&'a T),
    Owned(Rc<T>),
}
impl<T> AsRef<T> for LazyRc<'_, T>
where
    T: ?Sized,
{
    fn as_ref(&self) -> &T {
        match self {
            Self::Borrowed(t) => t,
            Self::Owned(t) => t,
        }
    }
}
impl<'a, T> Clone for LazyRc<'a, T>
where
    T: ?Sized,
{
    fn clone(&self) -> Self {
        match self {
            Self::Borrowed(borrow) => Self::Borrowed(borrow),
            Self::Owned(rc) => Self::Owned(Rc::clone(rc)),
        }
    }
}
impl<'a, T> From<Cow<'a, T>> for LazyRc<'a, T>
where
    T: ToOwned + ?Sized,
    Rc<T>: From<T::Owned>,
{
    fn from(cow: Cow<'a, T>) -> Self {
        match cow {
            Cow::Borrowed(borrow) => LazyRc::Borrowed(borrow),
            Cow::Owned(owned) => LazyRc::Owned(Rc::from(owned)),
        }
    }
}
impl<T> Deref for LazyRc<'_, T>
where
    T: ?Sized,
{
    type Target = T;

    fn deref(&self) -> &T {
        self.as_ref()
    }
}
impl<T> PartialEq for LazyRc<'_, T>
where
    T: PartialEq + ?Sized,
{
    fn eq(&self, r: &Self) -> bool {
        self.as_ref() == r.as_ref()
    }
}
