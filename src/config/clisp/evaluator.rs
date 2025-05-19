mod prelude;

use {
    crate::{
        config::clisp::{ast::Expr, lexer::Literal},
        cow::NonStaticCow,
    },
    dyn_clone::DynClone,
    nonempty_collections::{
        iter::{IntoNonEmptyIterator, NonEmptyIterator},
        vector::NEVec,
    },
    std::{
        any::{Any, type_name},
        borrow::Cow,
        collections::{HashMap, VecDeque},
        fmt::{self, Debug, Formatter},
        ops::RangeFrom,
        rc::Rc,
        vec,
    },
};

#[derive(Clone)]
pub struct Environment<'a>(NEVec<HashMap<&'a str, Value<'a>>>);
impl<'src> Environment<'src> {
    pub fn new() -> Self {
        Self(NEVec::new(prelude::new()))
    }
    pub fn clear(&mut self) {
        while self.0.pop().is_some() {}
    }
    pub fn pop(&mut self) {
        self.0.pop();
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
            Expr::List(mut apply) => {
                let Value::Fn(func) = apply
                    .pop_front()
                    .ok_or(EvalError::EmptyApply)
                    .and_then(|expr| self.eval(expr))
                    .map(Cow::into_owned)?
                else {
                    return Err(EvalError::NotAFunction);
                };

                self.0.push(HashMap::new());
                // let output = func(self, apply).map(Cow::Owned);
                self.0.pop();

                todo!()
            }
        }
    }

    pub fn get<'env>(&'env self, ident: &'src str) -> Option<&'env Value<'src>> {
        self.0.iter().rev().find_map(|vars| vars.get(ident))
    }
}

#[derive(Debug)]
pub enum EvalError<'a> {
    EmptyApply,
    NonIdentBinding(Expr<'a>),
    NoBody,
    NoBindings,
    NotAFunction,
    NotFound(&'a str),
    WrongType(TryFromValueError<'a>),
    WrongArity(usize),
    WrongVariadicArity(RangeFrom<usize>),
}
impl<'a> From<TryFromValueError<'a>> for EvalError<'a> {
    fn from(err: TryFromValueError<'a>) -> Self {
        Self::WrongType(err)
    }
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
    pub enum Value {
        Bool(bool),
        Int(i32),
        String(Cow<'a, Cow<'a, str>>),
        List(Rc<List<'a>>),
        Fn(NonStaticCow<'static, dyn ClispFn>),
        Dyn(Box<dyn DynValue>),
    }
}
impl Clone for Value<'_> {
    fn clone(&self) -> Self {
        match self {
            Self::Bool(b) => Self::Bool(*b),
            Self::Int(i) => Self::Int(*i),
            Self::String(s) => Self::String(s.clone()),
            Self::List(l) => Self::List(Rc::clone(l)),
            Self::Fn(f) => Self::Fn(f.clone()),
            Self::Dyn(v) => Self::Dyn(v.clone()),
        }
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

pub trait ClispFn:
    DynClone
    + for<'env, 'src> Fn(
        &'env mut Environment<'src>,
        VecDeque<Expr<'src>>,
    ) -> Result<Value<'src>, EvalError<'src>>
{
}
dyn_clone::clone_trait_object!(ClispFn);
impl<T> ClispFn for T where
    T: DynClone
        + for<'env, 'src> Fn(
            &'env mut Environment<'src>,
            VecDeque<Expr<'src>>,
        ) -> Result<Value<'src>, EvalError<'src>>
{
}
impl ToOwned for dyn ClispFn + '_ {
    type Owned = Box<Self>;

    fn to_owned(&self) -> Self::Owned {
        dyn_clone::clone_box(self)
    }
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
