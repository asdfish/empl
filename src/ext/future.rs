pub trait FutureExt: Future + Sized {
    /// Create a morphism between [Future::Output] and `O`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use empl::ext::future::FutureExt;
    /// # use std::io;
    /// # use tokio::runtime;
    /// # runtime::Builder::new_current_thread().build()?.block_on(async {
    /// assert_eq!(async { 1 }.pipe(|i| i + 1).await, 2);
    /// # });
    /// # Ok::<(), io::Error>(())
    /// ```
    fn pipe<F, O>(self, f: F) -> impl Future<Output = O>
    where
        F: FnOnce(Self::Output) -> O,
    {
        async { f(self.await) }
    }
}
impl<T> FutureExt for T where T: Future {}
