use {
    crate::ext::command::CommandChain,
    bumpalo::Bump,
    pin_project_lite::pin_project,
    std::{
        io,
        marker::Unpin,
        pin::Pin,
        task::{Context, Poll},
    },
    tokio::io::AsyncWriteExt,
};

macro_rules! decl_either {
    (($either_ident:ident, $either_future_ident:ident), [$(($names_snake:ident, $names_pascal:ident, $generics:ident, $out_generics:ident)),* $(,)?]) => {
        #[derive(Clone, Copy, Debug)]
        pub enum $either_ident<$($generics),*> {
            $($names_pascal($generics)),*
        }

        impl<$($generics),*> CommandChain for $either_ident<$($generics),*>
        where $($generics: CommandChain),* {
            async fn execute<W>(self, alloc: &Bump, out: &mut W) -> Result<(), io::Error>
            where
            W: AsyncWriteExt + Unpin,
            {
                match self {
                    $(Self::$names_pascal(cmd) => cmd.execute(alloc, out).await),*
                }
            }
        }

        pin_project! {
            #[derive(Clone, Copy, Debug)]
            pub struct $either_future_ident<$($generics),*> {
                $(#[pin] $names_snake: $generics),*
            }
        }
        impl<$($generics, $out_generics),*> $either_future_ident<$($generics),*>
        where $($generics: Future<Output = $out_generics>),* {
            #[allow(clippy::too_many_arguments)]
            pub const fn new($($names_snake: $generics),*) -> Self {
                Self {
                    $($names_snake),*
                }
            }
        }
        impl<$($generics, $out_generics),*> Future for $either_future_ident<$($generics),*>
        where $($generics: Future<Output = $out_generics>),* {
            type Output = $either_ident<$($out_generics),*>;

            fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
                let this = self.project();
                $(if let Poll::Ready(output) = this.$names_snake.poll(ctx) {
                    return Poll::Ready($either_ident::$names_pascal(output));
                })*

                Poll::Pending
            }
        }
    }
}
decl_either!(
    (Either, EitherFuture),
    [(left, Left, A, AO), (right, Right, B, BO)]
);
decl_either!(
    (Either3, EitherFuture3),
    [
        (first, First, A, AO),
        (second, Second, B, BO),
        (third, Third, C, CO)
    ]
);
decl_either!(
    (Either4, EitherFuture4),
    [
        (first, First, A, AO),
        (second, Second, B, BO),
        (third, Third, C, CO),
        (fourth, Fourth, D, DO)
    ]
);
decl_either!(
    (Either5, EitherFuture5),
    [
        (first, First, A, AO),
        (second, Second, B, BO),
        (third, Third, C, CO),
        (fourth, Fourth, D, DO),
        (fifth, Fifth, E, EO)
    ]
);
decl_either!(
    (Either6, EitherFuture6),
    [
        (first, First, A, AO),
        (second, Second, B, BO),
        (third, Third, C, CO),
        (fourth, Fourth, D, DO),
        (fifth, Fifth, E, EO),
        (sixth, Sixth, F, FO)
    ]
);
decl_either!(
    (Either7, EitherFuture7),
    [
        (first, First, A, AO),
        (second, Second, B, BO),
        (third, Third, C, CO),
        (fourth, Fourth, D, DO),
        (fifth, Fifth, E, EO),
        (sixth, Sixth, F, FO),
        (seventh, Seventh, G, GO)
    ]
);
decl_either!(
    (Either8, EitherFuture8),
    [
        (first, First, A, AO),
        (second, Second, B, BO),
        (third, Third, C, CO),
        (fourth, Fourth, D, DO),
        (fifth, Fifth, E, EO),
        (sixth, Sixth, F, FO),
        (seventh, Seventh, G, GO),
        (eighth, Eighth, H, HO)
    ]
);
decl_either!(
    (Either9, EitherFuture9),
    [
        (first, First, A, AO),
        (second, Second, B, BO),
        (third, Third, C, CO),
        (fourth, Fourth, D, DO),
        (fifth, Fifth, E, EO),
        (sixth, Sixth, F, FO),
        (seventh, Seventh, G, GO),
        (eighth, Eighth, H, HO),
        (nineth, Nineth, I, IO)
    ]
);
decl_either!(
    (Either10, EitherFuture10),
    [
        (first, First, A, AO),
        (second, Second, B, BO),
        (third, Third, C, CO),
        (fourth, Fourth, D, DO),
        (fifth, Fifth, E, EO),
        (sixth, Sixth, F, FO),
        (seventh, Seventh, G, GO),
        (eighth, Eighth, H, HO),
        (nineth, Nineth, I, IO),
        (tenth, Tenth, J, JO)
    ]
);
decl_either!(
    (Either11, EitherFuture11),
    [
        (first, First, A, AO),
        (second, Second, B, BO),
        (third, Third, C, CO),
        (fourth, Fourth, D, DO),
        (fifth, Fifth, E, EO),
        (sixth, Sixth, F, FO),
        (seventh, Seventh, G, GO),
        (eighth, Eighth, H, HO),
        (nineth, Nineth, I, IO),
        (tenth, Tenth, J, JO),
        (eleventh, Eleventh, K, KO)
    ]
);
decl_either!(
    (Either12, EitherFuture12),
    [
        (first, First, A, AO),
        (second, Second, B, BO),
        (third, Third, C, CO),
        (fourth, Fourth, D, DO),
        (fifth, Fifth, E, EO),
        (sixth, Sixth, F, FO),
        (seventh, Seventh, G, GO),
        (eighth, Eighth, H, HO),
        (nineth, Nineth, I, IO),
        (tenth, Tenth, J, JO),
        (eleventh, Eleventh, K, KO),
        (twelveth, Twelveth, L, LO)
    ]
);

#[derive(Clone, Copy, Debug)]
pub enum EitherOrBoth<L, R> {
    Left(L),
    Right(R),
    Both(L, R),
}
impl<L, R> EitherOrBoth<L, R> {
    pub fn new(l: Option<L>, r: Option<R>) -> Option<Self> {
        match (l, r) {
            (Some(l), Some(r)) => Some(Self::Both(l, r)),
            (Some(l), None) => Some(Self::Left(l)),
            (None, Some(r)) => Some(Self::Right(r)),
            (None, None) => None,
        }
    }
}
