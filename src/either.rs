use {
    crate::{
        config::clisp::parser::{Parsable, Parser, ParserError, ParserOutput},
        ext::command::CommandChain,
    },
    bumpalo::Bump,
    pin_project_lite::pin_project,
    std::{
        error::Error,
        fmt::{self, Debug, Display, Formatter},
        io,
        marker::{PhantomData, Unpin},
        pin::Pin,
        task::{Context, Poll},
    },
    tokio::io::AsyncWriteExt,
};

macro_rules! decl_either {
    (
        ($either_ident:ident, $either_future_ident:ident, $either_parser_ident:ident),
        [$(($names_snake:ident, $names_pascal:ident, $generics:ident)),* $(,)? ],
        ($last_name_snake:ident, $last_name_pascal:ident, $last_generic:ident)
    ) => {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum $either_ident<$($generics,)* $last_generic> {
            $($names_pascal($generics),)*
            $last_name_pascal($last_generic)
        }
        impl<$($generics,)* $last_generic> Display for $either_ident<$($generics,)* $last_generic>
        where $($generics: Display,)*
            $last_generic: Display
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
                match self {
                    Self::$last_name_pascal(d) => d.fmt(f),
                    $(Self::$names_pascal(d) => d.fmt(f)),*
                }
            }
        }
        impl<$($generics,)* $last_generic> Error for $either_ident<$($generics,)* $last_generic>
        where $($generics: Debug + Display,)* $last_generic: Debug + Display {}

        impl<$($generics,)* $last_generic> CommandChain for $either_ident<$($generics,)* $last_generic>
        where $($generics: CommandChain,)*
            $last_generic: CommandChain
        {
            async fn execute<W>(self, alloc: &Bump, out: &mut W) -> Result<(), io::Error>
            where
            W: AsyncWriteExt + Unpin,
            {
                match self {
                    Self::$last_name_pascal(cmd) => cmd.execute(alloc, out).await,
                    $(Self::$names_pascal(cmd) => cmd.execute(alloc, out).await),*
                }
            }
        }

        pin_project! {
            #[derive(Clone, Copy, Debug)]
            pub struct $either_future_ident<$($generics,)* $last_generic> {
                $(#[pin] $names_snake: $generics,)*
                #[pin] $last_name_snake: $last_generic,
            }
        }
        impl<$($generics,)* $last_generic> $either_future_ident<$($generics,)* $last_generic>
        where $($generics: Future,)* $last_generic: Future {
            #[allow(clippy::too_many_arguments)]
            pub const fn new($($names_snake: $generics,)* $last_name_snake: $last_generic) -> Self {
                Self {
                    $last_name_snake,
                    $($names_snake),*
                }
            }
        }
        impl<$($generics,)* $last_generic> Future for $either_future_ident<$($generics,)* $last_generic>
        where $($generics: Future,)* $last_generic: Future {
            type Output = $either_ident<$($generics::Output,)* $last_generic::Output>;

            fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
                let this = self.project();
                macro_rules! check_ready {
                    ($ident_snake:ident, $ident_pascal:ident) => {
                        if let Poll::Ready(output) = this.$ident_snake.poll(ctx) {
                            return Poll::Ready($either_ident::$ident_pascal(output));
                        }     
                    }
                }
                $(check_ready!($names_snake, $names_pascal);)*
                check_ready!($last_name_snake, $last_name_pascal);

                Poll::Pending
            }
        }

        #[derive(Clone, Copy, Debug)]
        pub struct $either_parser_ident<'a, Input, $($generics,)* $last_generic>
        where
            Input: Parsable<'a>,
            $($generics: Parser<'a, Input>,)*
            $last_generic: Parser<'a, Input>
        {
            _marker: PhantomData<&'a Input>,
            $($names_snake: $generics,)*
            $last_name_snake: $last_generic,
        }

        impl<'a, Input, $($generics,)* $last_generic> $either_parser_ident<'a, Input, $($generics,)* $last_generic>
        where
            Input: Parsable<'a>,
            $($generics: Parser<'a, Input>,)*
            $last_generic: Parser<'a, Input>
        {
            #[allow(clippy::too_many_arguments)]
            pub const fn new($($names_snake: $generics,)* $last_name_snake: $last_generic) -> Self {
                Self {
                    _marker: PhantomData,
                    $last_name_snake,
                    $($names_snake),*
                }
            }
        }

        impl<'a, Input, $($generics,)* $last_generic> Parser<'a, Input> for $either_parser_ident<'a, Input, $($generics,)* $last_generic>
        where
            Input: Parsable<'a>,
            $($generics: Parser<'a, Input>,)*
            $last_generic: Parser<'a, Input>
        {
            type Output = $either_ident<$($generics::Output,)* $last_generic::Output>;
            type Error = $last_generic::Error;

            fn parse(
                self,
                input: Input,
            ) -> Result<ParserOutput<'a, Input, Self::Output>, ParserError<Input::Item, Self::Error>> {
                $(if let Ok(po) = self.$names_snake.parse(input).map(|po| po.map_output($either_ident::$names_pascal)) {
                    return Ok(po);
                })*

                self.$last_name_snake.parse(input)
                    .map(|po| po.map_output($either_ident::$last_name_pascal))
            }
        }
    }
}
decl_either!(
    (Either, EitherFuture, EitherParser),
    [(left, Left, A)],
    (right, Right, B)
);
decl_either!(
    (Either3, EitherFuture3, EitherParser3),
    [
        (first, First, A),
        (second, Second, B)
    ],
    (third, Third, C)
);
decl_either!(
    (Either4, EitherFuture4, EitherParser4),
    [
        (first, First, A),
        (second, Second, B),
        (third, Third, C)
    ],
    (fourth, Fourth, D)
);
decl_either!(
    (Either5, EitherFuture5, EitherParser5),
    [
        (first, First, A),
        (second, Second, B),
        (third, Third, C),
        (fourth, Fourth, D)
    ],
    (fifth, Fifth, E)
);
decl_either!(
    (Either6, EitherFuture6, EitherParser6),
    [
        (first, First, A),
        (second, Second, B),
        (third, Third, C),
        (fourth, Fourth, D),
        (fifth, Fifth, E)
    ],
    (sixth, Sixth, F)
);
decl_either!(
    (Either7, EitherFuture7, EitherParser7),
    [
        (first, First, A),
        (second, Second, B),
        (third, Third, C),
        (fourth, Fourth, D),
        (fifth, Fifth, E),
        (sixth, Sixth, F)
    ],
    (seventh, Seventh, G)
);
decl_either!(
    (Either8, EitherFuture8, EitherParser8),
    [
        (first, First, A),
        (second, Second, B),
        (third, Third, C),
        (fourth, Fourth, D),
        (fifth, Fifth, E),
        (sixth, Sixth, F),
        (seventh, Seventh, G)
    ],
    (eighth, Eighth, H)
);
decl_either!(
    (Either9, EitherFuture9, EitherParser9),
    [
        (first, First, A),
        (second, Second, B),
        (third, Third, C),
        (fourth, Fourth, D),
        (fifth, Fifth, E),
        (sixth, Sixth, F),
        (seventh, Seventh, G),
        (eighth, Eighth, H)
    ],
    (nineth, Nineth, I)
);
decl_either!(
    (Either10, EitherFuture10, EitherParser10),
    [
        (first, First, A),
        (second, Second, B),
        (third, Third, C),
        (fourth, Fourth, D),
        (fifth, Fifth, E),
        (sixth, Sixth, F),
        (seventh, Seventh, G),
        (eighth, Eighth, H),
        (nineth, Nineth, I)
    ],
    (tenth, Tenth, J)
);
decl_either!(
    (Either11, EitherFuture11, EitherParser11),
    [
        (first, First, A),
        (second, Second, B),
        (third, Third, C),
        (fourth, Fourth, D),
        (fifth, Fifth, E),
        (sixth, Sixth, F),
        (seventh, Seventh, G),
        (eighth, Eighth, H),
        (nineth, Nineth, I),
        (tenth, Tenth, J)
    ],
    (eleventh, Eleventh, K)
);
decl_either!(
    (Either12, EitherFuture12, EitherParser12),
    [
        (first, First, A),
        (second, Second, B),
        (third, Third, C),
        (fourth, Fourth, D),
        (fifth, Fifth, E),
        (sixth, Sixth, F),
        (seventh, Seventh, G),
        (eighth, Eighth, H),
        (nineth, Nineth, I),
        (tenth, Tenth, J),
        (eleventh, Eleventh, K)
    ],
    (twelveth, Twelveth, L)
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

    /// Lazy constructor that executes the left function first.
    ///
    /// Never constructs [Self::Right].
    ///
    /// Returns [None] if `l` returns [None].
    pub fn new_lazy_left<F, G>(l: F, r: G) -> Option<Self>
    where
        F: FnOnce() -> Option<L>,
        G: FnOnce() -> Option<R>,
    {
        let l = l()?;
        match r() {
            Some(r) => Some(Self::Both(l, r)),
            None => Some(Self::Left(l)),
        }
    }
}
