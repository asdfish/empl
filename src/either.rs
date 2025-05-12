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
        marker::Unpin,
        pin::Pin,
        task::{Context, Poll},
    },
    tokio::io::AsyncWriteExt,
};

macro_rules! decl_either {
    (
        ($either_ident:ident, $either_output:ident),
        [$(($snake:ident, $pascals:ident)),* $(,)? ],
        ($last_snake:ident, $last_pascal:ident)
    ) => {
        #[derive(Clone, Copy, Debug, PartialEq)]
        pub enum $either_ident<$($pascals,)* $last_pascal> {
            $($pascals($pascals),)*
            $last_pascal($last_pascal)
        }
        impl<$($pascals,)* $last_pascal> Display for $either_ident<$($pascals,)* $last_pascal>
        where $($pascals: Display,)*
            $last_pascal: Display
        {
            fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
                match self {
                    Self::$last_pascal(d) => d.fmt(f),
                    $(Self::$pascals(d) => d.fmt(f)),*
                }
            }
        }
        impl<$($pascals,)* $last_pascal> Error for $either_ident<$($pascals,)* $last_pascal>
        where $($pascals: Debug + Display,)* $last_pascal: Debug + Display {}

        impl<$($pascals,)* $last_pascal> CommandChain for $either_ident<$($pascals,)* $last_pascal>
        where $($pascals: CommandChain,)*
            $last_pascal: CommandChain
        {
            async fn execute<W>(self, alloc: &Bump, out: &mut W) -> Result<(), io::Error>
            where
            W: AsyncWriteExt + Unpin,
            {
                match self {
                    Self::$last_pascal(cmd) => cmd.execute(alloc, out).await,
                    $(Self::$pascals(cmd) => cmd.execute(alloc, out).await),*
                }
            }
        }

        pin_project! {
            #[derive(Clone, Copy, Debug)]
            pub struct $either_output<$($pascals,)* $last_pascal> {
                $(#[pin] $snake: $pascals,)*
                #[pin] $last_snake: $last_pascal,
            }
        }
        impl<$($pascals,)* $last_pascal> $either_output<$($pascals,)* $last_pascal>
        where $($pascals: Future,)* $last_pascal: Future {
            #[allow(clippy::too_many_arguments)]
            pub const fn new($($snake: $pascals,)* $last_snake: $last_pascal) -> Self {
                Self {
                    $last_snake,
                    $($snake),*
                }
            }
        }
        impl<$($pascals,)* $last_pascal> Future for $either_output<$($pascals,)* $last_pascal>
        where $($pascals: Future,)* $last_pascal: Future {
            type Output = $either_ident<$($pascals::Output,)* $last_pascal::Output>;

            fn poll(self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
                let this = self.project();
                macro_rules! check_ready {
                    ($ident_snake:ident, $ident_pascal:ident) => {
                        if let Poll::Ready(output) = this.$ident_snake.poll(ctx) {
                            return Poll::Ready($either_ident::$ident_pascal(output));
                        }
                    }
                }
                $(check_ready!($snake, $pascals);)*
                check_ready!($last_snake, $last_pascal);

                Poll::Pending
            }
        }

        impl<'a, Input, $($pascals,)* $last_pascal> Parser<'a, Input> for $either_output<$($pascals,)* $last_pascal>
        where
            Input: Parsable<'a>,
            $($pascals: Parser<'a, Input>,)*
            $last_pascal: Parser<'a, Input>
        {
            type Output = $either_ident<$($pascals::Output,)* $last_pascal::Output>;
            type Error = $last_pascal::Error;

            fn parse(
                self,
                input: Input,
            ) -> Result<ParserOutput<'a, Input, Self::Output>, ParserError<Input::Item, Self::Error>> {
                $(if let Ok(po) = self.$snake.parse(input).map(|po| po.map_output($either_ident::$pascals)) {
                    return Ok(po);
                })*

                self.$last_snake.parse(input)
                    .map(|po| po.map_output($either_ident::$last_pascal))
            }
        }
    }
}
decl_either!((Either, EitherOutput), [(left, Left)], (right, Right));
decl_either!(
    (Either3, EitherOutput3),
    [(first, First), (second, Second)],
    (third, Third)
);
decl_either!(
    (Either4, EitherOutput4),
    [(first, First), (second, Second), (third, Third)],
    (fourth, Fourth)
);
decl_either!(
    (Either5, EitherOutput5),
    [
        (first, First),
        (second, Second),
        (third, Third),
        (fourth, Fourth)
    ],
    (fifth, Fifth)
);
decl_either!(
    (Either6, EitherOutput6),
    [
        (first, First),
        (second, Second),
        (third, Third),
        (fourth, Fourth),
        (fifth, Fifth)
    ],
    (sixth, Sixth)
);
decl_either!(
    (Either7, EitherOutput7),
    [
        (first, First),
        (second, Second),
        (third, Third),
        (fourth, Fourth),
        (fifth, Fifth),
        (sixth, Sixth)
    ],
    (seventh, Seventh)
);
decl_either!(
    (Either8, EitherOutput8),
    [
        (first, First),
        (second, Second),
        (third, Third),
        (fourth, Fourth),
        (fifth, Fifth),
        (sixth, Sixth),
        (seventh, Seventh)
    ],
    (eighth, Eighth)
);
decl_either!(
    (Either9, EitherOutput9),
    [
        (first, First),
        (second, Second),
        (third, Third),
        (fourth, Fourth),
        (fifth, Fifth),
        (sixth, Sixth),
        (seventh, Seventh),
        (eighth, Eighth)
    ],
    (nineth, Nineth)
);
decl_either!(
    (Either10, EitherOutput10),
    [
        (first, First),
        (second, Second),
        (third, Third),
        (fourth, Fourth),
        (fifth, Fifth),
        (sixth, Sixth),
        (seventh, Seventh),
        (eighth, Eighth),
        (nineth, Nineth)
    ],
    (tenth, Tenth)
);
decl_either!(
    (Either11, EitherOutput11),
    [
        (first, First),
        (second, Second),
        (third, Third),
        (fourth, Fourth),
        (fifth, Fifth),
        (sixth, Sixth),
        (seventh, Seventh),
        (eighth, Eighth),
        (nineth, Nineth),
        (tenth, Tenth)
    ],
    (eleventh, Eleventh)
);
decl_either!(
    (Either12, EitherOutput12),
    [
        (first, First),
        (second, Second),
        (third, Third),
        (fourth, Fourth),
        (fifth, Fifth),
        (sixth, Sixth),
        (seventh, Seventh),
        (eighth, Eighth),
        (nineth, Nineth),
        (tenth, Tenth),
        (eleventh, Eleventh)
    ],
    (twelveth, Twelveth)
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
