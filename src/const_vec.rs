use std::{
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    slice,
};

/// `T` must be [Copy] since dropping is not possible in const contexts.
#[derive(Clone, Copy)]
pub struct CVec<T, const N: usize>
where
    T: Copy,
{
    buffer: [MaybeUninit<T>; N],
    len: usize,
}
impl<T, const N: usize> CVec<T, N>
where
    T: Copy,
{
    pub const fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts((&raw const self.buffer).cast(), self.len) }
    }
    pub const fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut((&raw mut self.buffer).cast(), self.len) }
    }

    /// Concatenate 2 slices.
    ///
    /// # Panics
    ///
    /// Will panic if `with` + `self.len` is larger than `N`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::const_vec::CVec;
    /// let mut items = CVec::<u32, 6>::new();
    /// items.concat(&[1, 2, 3]);
    /// items.concat(&[4, 5, 6]);
    /// assert_eq!(items.as_ref(), &[1, 2, 3, 4, 5, 6]);
    /// ```
    ///
    /// ```should_panic
    /// # use empl::const_vec::CVec;
    /// let mut items = CVec::<u32, 5>::new();
    /// items.concat(&[1, 2, 3]);
    /// items.concat(&[4, 5, 6]);
    /// ```
    pub const fn concat(&mut self, with: &[T]) {
        assert!(self.len + with.len() <= N);
        unsafe {
            with.as_ptr().copy_to(
                self.buffer.as_mut_ptr().cast::<T>().add(self.len),
                with.len(),
            )
        };
        self.len += with.len();
    }

    pub const fn new() -> Self {
        Self {
            buffer: [const { MaybeUninit::uninit() }; N],
            len: 0,
        }
    }
    /// Push `item` into the end of the buffer.
    ///
    /// # Panics
    ///
    /// This function will panic if there is not enough capacity.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::const_vec::CVec;
    /// let mut nums = CVec::<u32, 1>::new();
    /// nums.push(1);
    /// assert_eq!(nums.as_ref(), &[1]);
    /// ```
    ///
    /// ```should_panic
    /// # use empl::const_vec::CVec;
    /// let mut nums = CVec::<u32, 1>::new();
    /// nums.push(1);
    /// nums.push(2);
    /// ```
    pub const fn push(&mut self, item: T) {
        assert!(self.len < N);
        self.buffer[self.len].write(item);
        self.len += 1;
    }
}
impl<T, const N: usize> AsRef<[T]> for CVec<T, N>
where
    T: Copy,
{
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}
impl<T, const N: usize> AsMut<[T]> for CVec<T, N>
where
    T: Copy,
{
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}
impl<T, const N: usize> Deref for CVec<T, N>
where
    T: Copy,
{
    type Target = [T];
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}
impl<T, const N: usize> DerefMut for CVec<T, N>
where
    T: Copy,
{
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}
impl<T, const N: usize> Default for CVec<T, N>
where
    T: Copy,
{
    fn default() -> Self {
        const { Self::new() }
    }
}
