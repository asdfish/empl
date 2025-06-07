use std::ops::ControlFlow;

pub trait Pair<L, R> {
    fn fst(self) -> L;
    fn snd(self) -> R;

    fn map_fst<F, L2>(self, _: F) -> (L2, R)
    where
        F: FnOnce(L) -> L2;
    fn map_snd<F, R2>(self, _: F) -> (L, R2)
    where
        F: FnOnce(R) -> R2;
}
impl<L, R> Pair<L, R> for (L, R) {
    fn fst(self) -> L {
        self.0
    }
    fn snd(self) -> R {
        self.1
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

    fn transpose(self) -> Self::Hkt
    where
        Self: Sized,
    {
        <Self::Hkt as Hkt<(L, R), E>>::from_control_flow(Self::to_control_flow(self))
    }
    fn to_control_flow(self) -> ControlFlow<E, (L, R)>;
}
impl<L, R> BiTranspose<L, R> for (Option<L>, R) {
    type Hkt = Option<(L, R)>;

    fn to_control_flow(self) -> ControlFlow<(), (L, R)> {
        match self {
            (Some(l), r) => ControlFlow::Continue((l, r)),
            (None, _) => ControlFlow::Break(()),
        }
    }
}
impl<L, R> BiTranspose<L, R> for (L, Option<R>) {
    type Hkt = Option<(L, R)>;

    fn to_control_flow(self) -> ControlFlow<(), (L, R)> {
        match self {
            (l, Some(r)) => ControlFlow::Continue((l, r)),
            (_, None) => ControlFlow::Break(()),
        }
    }
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
impl<L, R, E> BiTranspose<L, R, E> for (Result<L, E>, R) {
    type Hkt = Result<(L, R), E>;

    fn to_control_flow(self) -> ControlFlow<E, (L, R)> {
        match self {
            (Ok(l), r) => ControlFlow::Continue((l, r)),
            (Err(e), _) => ControlFlow::Break(e),
        }
    }
}
impl<L, R, E> BiTranspose<L, R, E> for (L, Result<R, E>) {
    type Hkt = Result<(L, R), E>;

    fn to_control_flow(self) -> ControlFlow<E, (L, R)> {
        match self {
            (l, Ok(r)) => ControlFlow::Continue((l, r)),
            (_, Err(e)) => ControlFlow::Break(e),
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
trait Hkt<T, E = ()> {
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
