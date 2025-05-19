use {
    crate::{
        either::EitherOrBoth,
        ext::command::{CommandChain, CommandIter},
    },
    std::{
        cmp::{Ordering, max},
        iter::FusedIterator,
        mem::MaybeUninit,
        ops::ControlFlow,
    },
};

pub trait IteratorExt: Iterator {
    fn adapt(self) -> CommandIter<Self, Self::Item>
    where
        Self: Sized,
        Self::Item: CommandChain,
    {
        CommandIter(self)
    }

    /// Collect the output of the iterator as an array.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::ext::iterator::IteratorExt;
    /// assert_eq!((0..3).collect_array::<3>(), Some([0, 1, 2]));
    /// assert_eq!((0..3).collect_array::<2>(), None);
    /// assert_eq!((0..3).collect_array::<4>(), None);
    /// ```
    fn collect_array<const N: usize>(self) -> Option<[Self::Item; N]>
    where
        Self: FusedIterator + Sized,
    {
        let mut output = [(); N].map(|_| MaybeUninit::uninit());
        let mut written = 0;
        self.enumerate()
            .try_for_each(|(i, val)| {
                if let Some(slot) = output.get_mut(i) {
                    slot.write(val);
                    written = i + 1;
                    Ok(())
                } else {
                    Err(())
                }
            })
            .ok()
            .filter(|_| written == N)?;

        Some(output.map(|item| unsafe { item.assume_init() }))
    }

    /// `Order` the items in an iterator by how many items are the same.
    ///
    /// # Examples
    ///
    /// ```
    /// # use {
    /// #     empl::ext::iterator::IteratorExt,
    /// #     std::cmp::Ordering,
    /// # };
    /// assert_eq!([1].into_iter().containment([1, 1]), Some(Ordering::Less));
    /// assert_eq!([1, 1].into_iter().containment([1]), Some(Ordering::Greater));
    /// assert_eq!([1].into_iter().containment([1]), Some(Ordering::Equal));
    /// ```
    fn containment<R, T>(self, r: R) -> Option<Ordering>
    where
        Self: Sized,
        Self::Item: PartialEq<T>,
        R: IntoIterator<Item = T>,
    {
        match self
            .zip_all(r)
            .try_fold(Some(Ordering::Equal), |_, items| match items {
                EitherOrBoth::Left(_) => ControlFlow::Break(Some(Ordering::Greater)),
                EitherOrBoth::Right(_) => ControlFlow::Break(Some(Ordering::Less)),
                EitherOrBoth::Both(l, r) if l == r => ControlFlow::Continue(Some(Ordering::Equal)),
                EitherOrBoth::Both(..) => ControlFlow::Break(None),
            }) {
            ControlFlow::Continue(v) | ControlFlow::Break(v) => v,
        }
    }

    fn zip_all<I, R>(self, r: I) -> ZipAll<Self, R>
    where
        Self: Sized,
        I: IntoIterator<IntoIter = R>,
        R: Iterator,
    {
        ZipAll {
            l: self,
            r: r.into_iter(),
        }
    }
}
impl<I> IteratorExt for I where I: Iterator {}

#[derive(Clone, Debug)]
pub struct ZipAll<L, R>
where
    L: Iterator,
    R: Iterator,
{
    l: L,
    r: R,
}
impl<A, B, L, R> Iterator for ZipAll<L, R>
where
    L: Iterator<Item = A>,
    R: Iterator<Item = B>,
{
    type Item = EitherOrBoth<A, B>;

    fn next(&mut self) -> Option<EitherOrBoth<A, B>> {
        EitherOrBoth::new(self.l.next(), self.r.next())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (l_lower, l_upper) = self.l.size_hint();
        let (r_lower, r_upper) = self.r.size_hint();

        (max(l_lower, r_lower), max(l_upper, r_upper))
    }
}
impl<A, B, L, R> DoubleEndedIterator for ZipAll<L, R>
where
    L: DoubleEndedIterator + Iterator<Item = A>,
    R: DoubleEndedIterator + Iterator<Item = B>,
{
    fn next_back(&mut self) -> Option<EitherOrBoth<A, B>> {
        EitherOrBoth::new(self.l.next_back(), self.r.next_back())
    }
}
impl<L, R> FusedIterator for ZipAll<L, R>
where
    L: Iterator + FusedIterator,
    R: Iterator + FusedIterator,
{
}
