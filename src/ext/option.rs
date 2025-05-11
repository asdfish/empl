use {
    pin_project_lite::pin_project,
    std::{
        future::Future,
        marker::Unpin,
        pin::Pin,
        task::{Context, Poll},
    },
};

pub trait OptionExt<T>: Sized {
    fn maybe_future(self) -> MaybeFuture<T>
    where T: Future;
}
impl<T> OptionExt<T> for Option<T> {
    fn maybe_future(self) -> MaybeFuture<T>
    where T: Future {
        MaybeFuture {
            future: self,
        }
    }
}

pin_project! {
    #[derive(Clone, Copy, Debug)]
    pub struct MaybeFuture<F> {
        future: Option<F>
    }
}
impl<F, T> Future for MaybeFuture<F>
where
    F: Future<Output = T> + Unpin,
{
    type Output = Option<T>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Option<T>> {
        let this = self.project();
        if let Some(future) = this.future {
            Pin::new(future).poll(ctx).map(Some)
        } else {
            Poll::Ready(None)
        }
    }
}
