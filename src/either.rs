use {
    crate::ext::command::CommandChain,
    bumpalo::Bump,
    std::{
        io,
        marker::Unpin,
    },
    tokio::io::AsyncWriteExt,
};

macro_rules! decl_either {
    ($ident:ident, [$(($names:ident, $generics:ident)),* $(,)?]) => {
        #[derive(Clone, Copy, Debug)]
        pub enum $ident<$($generics),*> {
            $($names($generics)),*
        }

        impl<$($generics),*> CommandChain for $ident<$($generics),*>
        where $($generics: CommandChain),* {
            async fn execute<W>(self, alloc: &Bump, out: &mut W) -> Result<(), io::Error>
            where
            W: AsyncWriteExt + Unpin,
            {
                match self {
                    $(Self::$names(cmd) => cmd.execute(alloc, out).await),*
                }
            }
        }
    }
}
decl_either!(Either4, [
    (First, A),
    (Second, B),
    (Third, C),
    (Fourth, D),
]);

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
