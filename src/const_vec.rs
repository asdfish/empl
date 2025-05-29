use std::{
    fmt::{self, Display, Formatter},
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    slice,
};

/// `T` must be [Copy] since dropping is not possible in const contexts.
#[derive(Clone, Copy)]
pub struct ConstVec<T, const N: usize>
where
    T: Copy,
{
    buffer: [MaybeUninit<T>; N],
    len: usize,
}
impl<T, const N: usize> ConstVec<T, N>
where
    T: Copy,
{
    pub const fn as_slice(&self) -> &[T] {
        unsafe { slice::from_raw_parts((&raw const self.buffer).cast(), self.len) }
    }
    pub const fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { slice::from_raw_parts_mut((&raw mut self.buffer).cast(), self.len) }
    }

    pub const fn capacity(&self) -> usize {
        N - self.len
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
    /// # use empl::const_vec::ConstVec;
    /// let mut items = ConstVec::<u32, 6>::new();
    /// items.concat(&[1, 2, 3]);
    /// items.concat(&[4, 5, 6]);
    /// assert_eq!(items.as_ref(), &[1, 2, 3, 4, 5, 6]);
    /// ```
    ///
    /// ```should_panic
    /// # use empl::const_vec::ConstVec;
    /// let mut items = ConstVec::<u32, 5>::new();
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
    /// # use empl::const_vec::ConstVec;
    /// let mut nums = ConstVec::<u32, 1>::new();
    /// nums.push(1);
    /// assert_eq!(nums.as_ref(), &[1]);
    /// ```
    ///
    /// ```should_panic
    /// # use empl::const_vec::ConstVec;
    /// let mut nums = ConstVec::<u32, 1>::new();
    /// nums.push(1);
    /// nums.push(2);
    /// ```
    pub const fn push(&mut self, item: T) {
        assert!(self.len < N);
        self.buffer[self.len].write(item);
        self.len += 1;
    }
}
impl<T, const N: usize> AsRef<[T]> for ConstVec<T, N>
where
    T: Copy,
{
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}
impl<T, const N: usize> AsMut<[T]> for ConstVec<T, N>
where
    T: Copy,
{
    fn as_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}
impl<T, const N: usize> Deref for ConstVec<T, N>
where
    T: Copy,
{
    type Target = [T];
    fn deref(&self) -> &[T] {
        self.as_slice()
    }
}
impl<T, const N: usize> DerefMut for ConstVec<T, N>
where
    T: Copy,
{
    fn deref_mut(&mut self) -> &mut [T] {
        self.as_mut_slice()
    }
}
impl<T, const N: usize> Default for ConstVec<T, N>
where
    T: Copy,
{
    fn default() -> Self {
        const { Self::new() }
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct ConstString<const N: usize>(ConstVec<u8, N>);
impl<const N: usize> ConstString<N> {
    pub const fn new() -> Self {
        Self(ConstVec::new())
    }

    /// Push a character
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::const_vec::ConstString;
    /// let mut string = ConstString::<11>::new();
    /// string.push_str("hello");
    /// string.push(' ');
    /// string.push_str("world");
    /// assert_eq!(string.as_ref(), "hello world");
    /// ```
    ///
    /// ```should_panic
    /// # use empl::const_vec::ConstString;
    /// let mut string = ConstString::<11>::new();
    /// string.push_str("hello");
    /// string.push(' ');
    /// string.push_str("world");
    /// string.push('!');
    /// ```
    pub const fn push(&mut self, ch: char) {
        assert!(self.0.capacity() >= ch.len_utf8());

        ch.encode_utf8(unsafe {
            slice::from_raw_parts_mut(
                self.0.buffer.as_mut_ptr().cast::<u8>().wrapping_add(self.0.len),
                self.0.capacity(),
            )
        });
        self.0.len += ch.len_utf8();
    }
    pub const fn push_str(&mut self, str: &str) {
        self.0.concat(str.as_bytes());
    }
}
impl<const N: usize> AsRef<str> for ConstString<N> {
    fn as_ref(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.0.as_ref()) }
    }
}
impl<const N: usize> AsMut<str> for ConstString<N> {
    fn as_mut(&mut self) -> &mut str {
        unsafe { str::from_utf8_unchecked_mut(self.0.as_mut()) }
    }
}
impl<const N: usize> Default for ConstString<N> {
    fn default() -> Self {
        const { Self::new() }
    }
}
impl<const N: usize> Deref for ConstString<N> {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_ref()
    }
}
impl<const N: usize> DerefMut for ConstString<N> {
    fn deref_mut(&mut self) -> &mut str {
        self.as_mut()
    }
}
impl<const N: usize> Display for ConstString<N> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        f.write_str(self.as_ref())
    }
}
