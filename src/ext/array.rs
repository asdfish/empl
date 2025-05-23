use std::mem::MaybeUninit;

pub trait ArrayExt<const N: usize, T, E> {
    fn transpose(self) -> Result<[T; N], E>
    where
        Self: Sized;
}

impl<const N: usize, T, E> ArrayExt<N, T, E> for [Result<T, E>; N] {
    fn transpose(self) -> Result<[T; N], E> {
        let mut output = [(); N].map(|_| MaybeUninit::uninit());
        output
            .iter_mut()
            .zip(self)
            .try_for_each(|(into, from)| {
                into.write(from?);
                Ok(())
            })
            .map(move |_| output.map(|slot| unsafe { slot.assume_init() }))
    }
}
