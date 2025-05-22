pub mod list;
mod prelude;

use {
    crate::{
        config::clisp::{ast::Expr, lexer::Literal},
        ext::{array::ArrayExt, iterator::IteratorExt},
    },
    crossterm::style::Color,
    dyn_clone::DynClone,
    nonempty_collections::vector::NEVec,
    std::{
        any::type_name,
        borrow::Cow,
        collections::{HashMap, VecDeque},
        env,
        fmt::{self, Debug, Display, Formatter},
        num::TryFromIntError,
        ops::{Range, RangeFrom},
        rc::Rc,
        str::FromStr,
    },
};

#[derive(Clone)]
pub struct Environment<'src>(NEVec<HashMap<&'src str, Value<'src>>>);
impl<'src> Environment<'src> {
    pub fn new() -> Self {
        Self(NEVec::new(prelude::new()))
    }
    pub fn with_symbols<I>(syms: I) -> Self
    where
        I: IntoIterator<Item = (&'src str, Value<'src>)>,
    {
        let mut output = Self::new();
        output.0.first_mut().extend(syms);

        output
    }
    pub fn last_mut(&mut self) -> &mut HashMap<&'src str, Value<'src>> {
        self.0.last_mut()
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
                let output = func(self, apply).map(Cow::Owned);
                self.0.pop();

                output
            }
            Expr::Value(value) => Ok(Cow::Owned(value)),
        }
    }
    pub fn eval_into<'env, T>(&'env mut self, expr: Expr<'src>) -> Result<T, EvalError<'src>>
    where
        T: TryFromValue<'src>,
    {
        self.eval(expr)
            .map(Cow::into_owned)
            .and_then(|expr| T::try_from_value(expr).map_err(EvalError::WrongType))
    }

    pub fn get<'env>(&'env self, ident: &'src str) -> Option<&'env Value<'src>> {
        self.0.iter().rev().find_map(|vars| vars.get(ident))
    }
}
impl<'src> Default for Environment<'src> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub enum Arity {
    Static(usize),
    RangeFrom(RangeFrom<usize>),
    Range(Range<usize>),
}

#[derive(Debug)]
pub enum EvalError<'a> {
    EmptyApply,
    MultipleBindings(&'a str),
    NonIdentBinding(Expr<'a>),
    EmptyListBindings,
    EnvVar(env::VarError),
    InvalidColor(InvalidColorError<'a>),
    NonListBindings(Expr<'a>),
    NoBindings,
    NoBody,
    NotAFunction,
    NotFound(&'a str),
    Overflow,
    UnknownCfgField(Cow<'a, Cow<'a, str>>),
    WrongType(TryFromValueError<'a>),
    WrongArity(Arity),
    WrongListArity(Arity),
    WrongBindingArity(Arity),
}
impl<'a> Display for EvalError<'a> {
    fn fmt(&self, _f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        // TODO: implement display
        Ok(())
    }
}
impl From<env::VarError> for EvalError<'_> {
    fn from(err: env::VarError) -> Self {
        Self::EnvVar(err)
    }
}
impl<'a> From<TryFromValueError<'a>> for EvalError<'a> {
    fn from(err: TryFromValueError<'a>) -> Self {
        Self::WrongType(err)
    }
}

macro_rules! impl_value_variant {
    ($variant:ident($ty:ty)) => {
        impl<'a> From<$ty> for Value<'a> {
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
        }
    };
}
#[derive(Clone, Default)]
pub enum Value<'src> {
    #[default]
    Unit,
    Bool(bool),
    Int(i32),
    String(Cow<'src, Cow<'src, str>>),
    List(Rc<List<'src>>),
    Fn(Rc<dyn ClispFn<'src> + 'src>),
}
impl_value_variant!(Bool(bool));
impl_value_variant!(Int(i32));
impl_value_variant!(String(Cow<'a, Cow<'a, str>>));
impl_value_variant!(List(Rc<List<'a>>));
impl_value_variant!(Fn(Rc<dyn ClispFn<'a> + 'a>));
impl Debug for Value<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Unit => f.debug_tuple("Unit").finish(),
            Self::Bool(b) => f.debug_tuple("Bool").field(b).finish(),
            Self::Int(i) => f.debug_tuple("Int").field(i).finish(),
            Self::String(s) => f.debug_tuple("String").field(s).finish(),
            Self::List(l) => f.debug_tuple("List").field(l).finish(),
            Self::Fn(_) => f.debug_tuple("Fn").finish_non_exhaustive(),
        }
    }
}
impl<'src> PartialEq for Value<'src> {
    fn eq(&self, r: &Self) -> bool {
        match (self, r) {
            (Self::Unit, Self::Unit) => true,
            (Self::Bool(l), Self::Bool(r)) => l == r,
            (Self::Int(l), Self::Int(r)) => l == r,
            (Self::String(l), Self::String(r)) => l == r,
            (Self::List(l), Self::List(r)) => l == r,
            _ => false,
        }
    }
}
impl<'src> TryFrom<Value<'src>> for Option<Color> {
    type Error = InvalidColorError<'src>;

    fn try_from(val: Value<'src>) -> Result<Option<Color>, InvalidColorError<'src>> {
        match val {
            Value::List(rgb) => {
                let [r, g, b] = rgb
                    .iter()
                    .map(|val| {
                        if let Value::Int(color) = val {
                            Ok(color)
                        } else {
                            Err(InvalidColorError::WrongListType(val))
                        }
                        .and_then(|color| {
                            u8::try_from(color).map_err(InvalidColorError::InvalidRgb)
                        })
                    })
                    .collect_array::<3>()
                    .ok_or(InvalidColorError::WrongListArity)?
                    .transpose()?;

                Ok(Some(Color::Rgb { r, g, b }))
            }
            Value::String(name) if name.as_ref().as_ref() == "none" => Ok(None),
            Value::String(name) => Color::from_str(name.as_ref().as_ref())
                .map_err(|_| InvalidColorError::UnknownColor(name))
                .map(Some),
            val => Err(InvalidColorError::WrongType(val)),
        }
    }
}
#[derive(Debug)]
pub enum InvalidColorError<'src> {
    InvalidRgb(TryFromIntError),
    WrongListArity,
    WrongType(Value<'src>),
    WrongListType(Value<'src>),
    UnknownColor(Cow<'src, Cow<'src, str>>),
}

pub trait ClispFn<'src>:
    DynClone + Fn(&mut Environment<'src>, VecDeque<Expr<'src>>) -> Result<Value<'src>, EvalError<'src>>
{
}
dyn_clone::clone_trait_object!(ClispFn<'_>);
impl<'src, T> ClispFn<'src> for T where
    T: DynClone
        + Fn(&mut Environment<'src>, VecDeque<Expr<'src>>) -> Result<Value<'src>, EvalError<'src>>
{
}
impl ToOwned for dyn ClispFn<'_> {
    type Owned = Rc<Self>;

    fn to_owned(&self) -> Self::Owned {
        dyn_clone::clone_box(self).into()
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

#[derive(Clone, Debug, PartialEq)]
pub enum List<'a> {
    Nil,
    Cons(Value<'a>, Rc<Self>),
}
impl<'a> List<'a> {
    pub fn new<C, I>(iter: C) -> Rc<List<'a>>
    where
        C: IntoIterator<IntoIter = I, Item = Value<'a>>,
        I: DoubleEndedIterator + Iterator<Item = Value<'a>>,
    {
        iter.into_iter()
            .rev()
            .fold(Rc::new(List::Nil), |cdr, val| Rc::new(List::Cons(val, cdr)))
    }

    pub fn iter(self: Rc<Self>) -> list::Iter<'a> {
        list::Iter(self)
    }
}

#[derive(Debug)]
pub struct TryFromValueError<'a>(Value<'a>, &'static str);
impl<'a> Display for TryFromValueError<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "`{:?}` is not of type `{}`", self.0, self.1)
    }
}
