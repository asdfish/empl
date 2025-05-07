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
