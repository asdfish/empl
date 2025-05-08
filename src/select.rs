use {
    pin_project_lite::pin_project,
    std::{
        future::Future,
        pin::Pin,
        task::{Context, Poll},
    },
};

macro_rules! decl_select {
    ($name_pascal:ident, [$(($generics_snake:ident, $generics_pascal:ident)),* $(,)?]) => {
        pin_project! {
            #[derive(Clone, Copy, Debug)]
            pub struct $name_pascal<O, $($generics_pascal),*>
            where $($generics_pascal: Future<Output = O>),* {
                $(#[pin] $generics_snake: $generics_pascal),*
            }
        }
        impl<O, $($generics_pascal),*> $name_pascal<O, $($generics_pascal),*>
        where $($generics_pascal: Future<Output = O>),* {
            #[allow(clippy::too_many_arguments)]
            pub const fn new($($generics_snake: $generics_pascal),*) -> Self {
                $name_pascal {
                    $($generics_snake),*
                }
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
decl_select!(Select, [(a, A), (b, B)]);
decl_select!(Select3, [(a, A), (b, B), (c, C)]);
decl_select!(Select4, [(a, A), (b, B), (c, C), (d, D)]);
decl_select!(Select5, [(a, A), (b, B), (c, C), (d, D), (e, E)]);
decl_select!(Select6, [(a, A), (b, B), (c, C), (d, D), (e, E), (f, F)]);
decl_select!(
    Select7,
    [(a, A), (b, B), (c, C), (d, D), (e, E), (f, F), (g, G)]
);
decl_select!(
    Select8,
    [
        (a, A),
        (b, B),
        (c, C),
        (d, D),
        (e, E),
        (f, F),
        (g, G),
        (h, H)
    ]
);
decl_select!(
    Select9,
    [
        (a, A),
        (b, B),
        (c, C),
        (d, D),
        (e, E),
        (f, F),
        (g, G),
        (h, H),
        (i, I)
    ]
);
decl_select!(
    Select10,
    [
        (a, A),
        (b, B),
        (c, C),
        (d, D),
        (e, E),
        (f, F),
        (g, G),
        (h, H),
        (i, I),
        (j, J)
    ]
);
decl_select!(
    Select11,
    [
        (a, A),
        (b, B),
        (c, C),
        (d, D),
        (e, E),
        (f, F),
        (g, G),
        (h, H),
        (i, I),
        (j, J),
        (k, K)
    ]
);
decl_select!(
    Select12,
    [
        (a, A),
        (b, B),
        (c, C),
        (d, D),
        (e, E),
        (f, F),
        (g, G),
        (h, H),
        (i, I),
        (j, J),
        (k, K),
        (l, L)
    ]
);
