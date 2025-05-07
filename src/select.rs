use {
    pin_project_lite::pin_project,
    std::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    },
};

macro_rules! decl_select {
    (($name_snake:ident, $name_pascal:ident), [$(($generics_snake:ident, $generics_pascal:ident)),* $(,)?]) => {
        pin_project! {
            #[derive(Clone, Copy, Debug)]
            pub struct $name_pascal<O, $($generics_pascal),*>
            where $($generics_pascal: Future<Output = O>),* {
                $(#[pin] $generics_snake: $generics_pascal),*
            }
        }
        pub const fn $name_snake<O, $($generics_pascal),*>($($generics_snake: $generics_pascal),*) -> $name_pascal<O, $($generics_pascal),*>
        where $($generics_pascal: Future<Output = O>),* {
            $name_pascal {
                $($generics_snake),*
            }
        }

        impl<O, $($generics_pascal),*> Future for $name_pascal<O, $($generics_pascal),*>
        where $($generics_pascal: Future<Output = O>),* {
            type Output = O;

            fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<O> {
                let this = self.project();

                $(if let output @ Poll::Ready(_) = this.$generics_snake.poll(ctx) {
                    return output;
                })*

                Poll::Pending
            }
        }
    }
}
decl_select!((select2, Select2), [(a, A), (b, B)]);
