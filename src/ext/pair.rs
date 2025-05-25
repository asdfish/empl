pub trait PairExt {
    fn fst<L, R>(self) -> L
    where
        Self: Into<(L, R)>,
    {
        self.into().0
    }
    fn snd<L, R>(self) -> R
    where
        Self: Into<(L, R)>,
    {
        self.into().1
    }

    fn map_fst<F, L, L2, R>(self, map: F) -> (L2, R)
    where
        Self: Into<(L, R)>,
        F: FnOnce(L) -> L2,
    {
        let (l, r) = self.into();
        (map(l), r)
    }
    fn map_snd<F, L, R, R2>(self, map: F) -> (L, R2)
    where
        Self: Into<(L, R)>,
        F: FnOnce(R) -> R2,
    {
        let (l, r) = self.into();
        (l, map(r))
    }

    fn transpose_fst<L, R, E>(self) -> Result<(L, R), E>
    where
        Self: Into<(Result<L, E>, R)>,
    {
        let (l, r) = self.into();

        l.map(move |l| (l, r))
    }
    fn transpose_snd<L, R, E>(self) -> Result<(L, R), E>
    where
        Self: Into<(L, Result<R, E>)>,
    {
        let (l, r) = self.into();

        r.map(move |r| (l, r))
    }
}
impl<T> PairExt for T {}
