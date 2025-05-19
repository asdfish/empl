mod prelude;

use {
    crate::config::clisp::{
        ast::Expr,
        lexer::Literal,
    },
    dyn_clone::DynClone,
    nonempty_collections::iter::{IntoNonEmptyIterator, NonEmptyIterator},
    std::{
        any::{Any, type_name},
        borrow::Cow,
        collections::HashMap,
        fmt::{self, Debug, Formatter},
        iter::{self, FusedIterator},
        rc::Rc,
    },
};

#[derive(Clone)]
pub struct Environment<'a>(Vec<HashMap<&'a str, Value<'a>>>);
impl<'src> Environment<'src> {
    pub fn new() -> Self {
        Self(Vec::from_iter(iter::once(prelude::new())))
    }

    pub fn eval<'env>(
        &'env mut self,
        expr: Expr<'src>,
    ) -> Result<Cow<'env, Value<'src>>, EvalError<'src>> {
        match expr {
            Expr::Literal(Literal::Bool(b)) => Ok(Cow::Owned(Value::Bool(*b))),
            Expr::Literal(Literal::Ident(id)) => self
                .get(id)
                .map(Cow::Borrowed)
                .ok_or(EvalError::NotFound(id)),
            Expr::Literal(Literal::Int(i)) => Ok(Cow::Owned(Value::Int(*i))),
            Expr::Literal(Literal::String(s)) => Ok(Cow::Owned(Value::String(Cow::Borrowed(s)))),
            Expr::Apply(apply) => {
                let apply = apply.into_nonempty_iter();
                let (func, args) = apply.next();
                let args = args
                    .map(|expr| self.eval(expr).map(Cow::into_owned))
                    .collect::<Result<Vec<_>, _>>()?;

                let func = match self.eval(func).map(Cow::into_owned)? {
                    Value::Fn(f) => f,
                    _ => return Err(EvalError::NotAFunction),
                };
                func(&mut args.into_iter().fuse())
                    .map(Cow::Owned)
                    .map_err(EvalError::Fn)
            }
        }
    }

    pub fn get<'env>(&'env self, ident: &'src str) -> Option<&'env Value<'src>> {
        self.0.iter().rev().find_map(|vars| vars.get(ident))
    }
}

#[derive(Debug)]
pub enum EvalError<'a> {
    NotAFunction,
    NotFound(&'a str),
    Fn(FnCallError<'a>),
}

pub trait DynValue: Any + DynClone {}
impl<T> DynValue for T where T: Any + DynClone {}
dyn_clone::clone_trait_object!(DynValue);

macro_rules! decl_value {
    (
    $(#[$attr:meta])*
    $vis:vis enum $ident:ident {
        $($variant:ident($ty:ty)),* $(,)?
    }) => {
        $(#[$attr]),*
        $vis enum $ident<'a> {
            $($variant($ty)),*
        }

        $(impl<'a> From<$ty> for $ident<'a> {
            fn from(val: $ty) -> Self {
                Self::$variant(val)
            }
        }

        impl<'a> TryFromValue<'a> for $ty {
            fn try_from_value(val: Value<'a>) -> Result<$ty, TryFromValueError<'a>> {
                match val {
                    Value::$variant(val) => Ok(val),
                    val => Err(TryFromValueError(val, type_name::<$ty>())),
                }
            }
        })*
    }
}
decl_value! {
    #[derive(Clone)]
    pub enum Value {
        Bool(bool),
        Int(i32),
        String(Cow<'a, Cow<'a, str>>),
        List(Rc<List<'a>>),
        Fn(Box<dyn ClispFn<'a>>),
        Dyn(Box<dyn DynValue>),
    }
}
impl Debug for Value<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Bool(b) => f.debug_tuple("Bool").field(b).finish(),
            Self::Int(i) => f.debug_tuple("Int").field(i).finish(),
            Self::String(s) => f.debug_tuple("String").field(s).finish(),
            Self::List(l) => f.debug_tuple("List").field(l).finish(),
            Self::Fn(_) => f.debug_tuple("Fn").finish_non_exhaustive(),
            Self::Dyn(_) => f.debug_tuple("Dyn").finish_non_exhaustive(),
        }
    }
}

#[derive(Debug)]
pub enum FnCallError<'a> {
    WrongType(TryFromValueError<'a>),
    WrongArity(usize),
}
impl<'a> From<TryFromValueError<'a>> for FnCallError<'a> {
    fn from(err: TryFromValueError<'a>) -> Self {
        Self::WrongType(err)
    }
}

pub trait Args<'a>: FusedIterator + Iterator<Item = Value<'a>> {}
impl<'a, T> Args<'a> for T where T: FusedIterator + Iterator<Item = Value<'a>> {}

pub trait ClispFn<'a>:
    DynClone + Fn(&mut dyn Args<'a>) -> Result<Value<'a>, FnCallError<'a>>
{
}
dyn_clone::clone_trait_object!(ClispFn<'_>);
impl<'a, T> ClispFn<'a> for T where
    T: Clone + Fn(&mut dyn Args<'a>) -> Result<Value<'a>, FnCallError<'a>>
{
}

pub trait TryFromValue<'a> {
    fn try_from_value(_: Value<'a>) -> Result<Self, TryFromValueError<'a>>
    where
        Self: Sized;
}
impl<'a, T> TryFromValue<'a> for T
where
    T: From<Value<'a>>,
{
    fn try_from_value(val: Value<'a>) -> Result<Self, TryFromValueError<'a>>
    where
        Self: Sized,
    {
        Ok(T::from(val))
    }
}

#[derive(Clone, Debug)]
pub enum List<'a> {
    Nil,
    Cons(Value<'a>, Rc<Self>),
}

#[derive(Debug)]
pub struct TryFromValueError<'a>(Value<'a>, &'static str);
