use std::ops::ControlFlow;

pub trait BiFunctor<L, R> {
    fn fst(self) -> L;
    fn snd(self) -> R;

    fn bimap<F, G, L2, R2>(self, f: F, g: G) -> (L2, R2)
    where
        Self: Sized,
        F: FnOnce(L) -> L2,
        G: FnOnce(R) -> R2,
    {
        self.map_fst(f).map_snd(g)
    }
    fn map_fst<F, L2>(self, _: F) -> (L2, R)
    where
        F: FnOnce(L) -> L2;
    fn map_snd<F, R2>(self, _: F) -> (L, R2)
    where
        F: FnOnce(R) -> R2;

    fn bi_map<F, G, L2, R2>(self, f: F, g: G) -> (L2, R2)
    where
        Self: Sized,
        F: FnOnce(L) -> L2,
        G: FnOnce(R) -> R2,
    {
        self.map_fst(f).map_snd(g)
    }
}
impl<L, R> BiFunctor<L, R> for (L, R) {
    fn fst(self) -> L {
        self.0
    }
    fn snd(self) -> R {
        self.1
    }

    fn bimap<F, G, L2, R2>(self, f: F, g: G) -> (L2, R2)
    where
        Self: Sized,
        F: FnOnce(L) -> L2,
        G: FnOnce(R) -> R2,
    {
        (f(self.0), g(self.1))
    }
    fn map_fst<F, L2>(self, morphism: F) -> (L2, R)
    where
        F: FnOnce(L) -> L2,
    {
        (morphism(self.0), self.1)
    }
    fn map_snd<F, R2>(self, morphism: F) -> (L, R2)
    where
        F: FnOnce(R) -> R2,
    {
        (self.0, morphism(self.1))
    }
}

pub trait BiTranspose<L, R, E = ()> {
    type Hkt: Hkt<(L, R), E>;

    fn bi_transpose(self) -> Self::Hkt
    where
        Self: Sized,
    {
        <Self::Hkt as Hkt<(L, R), E>>::from_control_flow(Self::to_control_flow(self))
    }
    fn to_control_flow(self) -> ControlFlow<E, (L, R)>;
}
impl<L, R> BiTranspose<L, R> for (Option<L>, Option<R>) {
    type Hkt = Option<(L, R)>;

    fn to_control_flow(self) -> ControlFlow<(), (L, R)> {
        match self {
            (Some(l), Some(r)) => ControlFlow::Continue((l, r)),
            _ => ControlFlow::Break(()),
        }
    }
}
impl<L, R, E> BiTranspose<L, R, E> for (Result<L, E>, Result<R, E>) {
    type Hkt = Result<(L, R), E>;

    fn to_control_flow(self) -> ControlFlow<E, (L, R)> {
        match self {
            (Ok(l), Ok(r)) => ControlFlow::Continue((l, r)),
            (Err(e), _) | (_, Err(e)) => ControlFlow::Break(e),
        }
    }
}
pub trait Hkt<T, E = ()> {
    fn from_control_flow(_: ControlFlow<E, T>) -> Self
    where
        Self: Sized;
}
impl<T, E> Hkt<T, E> for Result<T, E> {
    fn from_control_flow(flow: ControlFlow<E, T>) -> Self {
        match flow {
            ControlFlow::Break(e) => Err(e),
            ControlFlow::Continue(t) => Ok(t),
        }
    }
}
impl<T> Hkt<T> for Option<T> {
    fn from_control_flow(flow: ControlFlow<(), T>) -> Self {
        match flow {
            ControlFlow::Break(()) => None,
            ControlFlow::Continue(t) => Some(t),
        }
    }
}
