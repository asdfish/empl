pub mod function;

use {
    crate::config::clisp::{ast::Expr, lexer::Literal},
    dyn_clone::DynClone,
    std::{any::{Any, type_name}, borrow::Cow, collections::HashMap, rc::Rc},
};

#[derive(Clone)]
pub struct Environment<'a>(Vec<HashMap<&'a str, Value<'a>>>);
impl<'src> Environment<'src> {
    pub fn eval<'env>(&'env mut self, expr: Expr<'src>) -> Option<Cow<'env, Value<'src>>> {
        match expr {
            Expr::Literal(Literal::Bool(b)) => Some(Cow::Owned(Value::Bool(*b))),
            Expr::Literal(Literal::Ident(id)) => self.get(id).map(Cow::Borrowed),
            Expr::Literal(Literal::Int(i)) => Some(Cow::Owned(Value::Int(*i))),
            Expr::Literal(Literal::String(s)) => Some(Cow::Owned(Value::String(Cow::Borrowed(s)))),
            _ => todo!(),
        }
    }

    pub fn get<'env>(&'env self, ident: &'src str) -> Option<&'env Value<'src>> {
        self.0.iter().rev().find_map(|vars| vars.get(ident))
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
    #[derive(Clone)]
    pub enum Value {
        Bool(bool),
        Int(i32),
        String(Cow<'a, Cow<'a, str>>),
        List(Box<List<'a>>),
        Dyn(Box<dyn DynValue>),
    }
}

pub trait TryFromValue<'a> {
    fn try_from_value(_: Value<'a>) -> Result<Self, TryFromValueError<'a>>
    where Self: Sized;
}
impl<'a, T> TryFromValue<'a> for T
where T: From<Value<'a>> {
    fn try_from_value(val: Value<'a>) -> Result<Self, TryFromValueError<'a>>
    where Self: Sized {
        Ok(T::from(val))
    }
}

#[derive(Clone)]
pub enum List<'a> {
    Nil,
    Cons(Value<'a>, Rc<Self>),
}

pub struct TryFromValueError<'a>(Value<'a>, &'static str);
