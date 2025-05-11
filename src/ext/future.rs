pub trait FutureExt: Future + Sized {
    fn pipe<F, O>(self, f: F) -> impl Future<Output = O>
    where
        F: FnOnce(Self::Output) -> O,
    {
        async { f(self.await) }
    }
}
impl<T> FutureExt for T where T: Future {}
