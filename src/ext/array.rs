use std::mem::MaybeUninit;

pub trait ArrayExt<const N: usize, T, E> {
    fn transpose(self) -> Result<[T; N], E>
    where
        Self: Sized;
}

impl<const N: usize, T, E> ArrayExt<N, T, E> for [Result<T, E>; N] {
    fn transpose(self) -> Result<[T; N], E>
    where
        Self: Sized,
    {
        let mut output = [(); N].map(|_| MaybeUninit::uninit());
        self.into_iter()
            .enumerate()
            .try_for_each(|(i, item)| {
                output[i].write(item?);
                Ok(())
            })
            .map(move |_| output.map(|item| unsafe { item.assume_init() }))
    }
}
