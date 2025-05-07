use {
    pin_project_lite::pin_project,
    std::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    },
};

pub trait FutureExt: Future + Sized {
    fn pipe<P, O>(self, pipe: P) -> Pipe<Self, P, Self::Output, O> 
    where P: FnMut(Self::Output) -> O {
        Pipe {
            future: self,
            pipe
        }
    }
}
impl<T> FutureExt for T
where T: Future {}

pin_project! {
    #[derive(Clone, Copy, Debug)]
    pub struct Pipe<F, P, T, O>
    where
        F: Future<Output = T>,
        P: FnMut(T) -> O,
    {
        #[pin] future: F,
        pipe: P,
    }
}
impl<F, P, T, O> Future for Pipe<F, P, T, O>
where
    F: Future<Output = T>,
    P: FnMut(T) -> O
{
    type Output = O;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<O> {
        let this = self.project();
        this.future.poll(ctx).map(this.pipe)
    }
}
