pub mod list;
mod prelude;

use {
    crate::{
        config::{
            UnknownKeyActionError,
            clisp::{ast::Expr, lexer::Literal},
        },
        ext::{array::ArrayExt, iterator::IteratorExt},
        lazy_rc::LazyRc,
    },
    crossterm::style::Color,
    dyn_clone::DynClone,
    nonempty_collections::vector::NEVec,
    std::{
        any::type_name,
        collections::{HashMap, VecDeque},
        env,
        fmt::{self, Debug, Display, Formatter},
        io,
        num::TryFromIntError,
        ops::{Range, RangeFrom},
        path::Path,
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

    fn eval_inner<'env, Pre, Post>(
        &'env mut self,
        expr: Expr<'src>,
        pre: Pre,
        post: Post,
    ) -> Result<Value<'src>, EvalError<'src>>
    where
        Pre: FnOnce(&mut NEVec<HashMap<&'src str, Value<'src>>>),
        Post: FnOnce(&mut NEVec<HashMap<&'src str, Value<'src>>>),
    {
        match expr {
            Expr::Literal(Literal::Bool(b)) => Ok(Value::Bool(*b)),
            Expr::Literal(Literal::Ident(id)) => {
                self.get(id).cloned().ok_or(EvalError::NotFound(id))
            }
            Expr::Literal(Literal::Int(i)) => Ok(Value::Int(*i)),
            Expr::Literal(Literal::String(s)) => Ok(Value::String(LazyRc::Borrowed(s))),
            Expr::List(mut apply) => {
                let Value::Fn(func) = apply
                    .pop_front()
                    .ok_or(EvalError::EmptyApply)
                    .and_then(|expr| self.eval(expr))?
                else {
                    return Err(EvalError::NotAFunction);
                };

                pre(&mut self.0);
                let output = func(self, apply);
                post(&mut self.0);

                output
            }
            Expr::Value(value) => Ok(value),
        }
    }
    pub fn eval<'env>(&'env mut self, expr: Expr<'src>) -> Result<Value<'src>, EvalError<'src>> {
        self.eval_inner(
            expr,
            |frames| frames.push(HashMap::new()),
            |frames| {
                frames.pop();
            },
        )
    }
    pub fn eval_tail_call<'env>(
        &'env mut self,
        expr: Expr<'src>,
    ) -> Result<Value<'src>, EvalError<'src>> {
        self.eval_inner(expr, |_| {}, |_| {})
    }
    pub fn eval_into<'env, T>(&'env mut self, expr: Expr<'src>) -> Result<T, EvalError<'src>>
    where
        T: TryFromValue<'src>,
    {
        self.eval(expr)
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
pub enum EvalError<'src> {
    EmptyApply,
    MultipleBindings(&'src str),
    NonIdentBinding(Expr<'src>),
    EmptyListBindings,
    EnvVar(env::VarError),
    InvalidColor(InvalidColorError<'src>),
    Io(io::Error),
    NonListBindings(Expr<'src>),
    NoBindings,
    NoBody,
    NotAFunction,
    NotFound(&'src str),
    Overflow,
    UnknownCfgField(LazyRc<'src, str>),
    UnknownKeyAction(UnknownKeyActionError<'src>),
    UnknownKeyModifier(char),
    UnknownKeyCode(LazyRc<'src, str>),
    WrongType(TryFromValueError<'src>),
    WrongArity(Arity),
    WrongListArity(Arity),
    WrongBindingArity(Arity),
}
impl<'src> Display for EvalError<'src> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        // TODO: implement display properly
        write!(f, "{:?}", self)
    }
}
impl From<env::VarError> for EvalError<'_> {
    fn from(err: env::VarError) -> Self {
        Self::EnvVar(err)
    }
}
impl<'src> From<TryFromValueError<'src>> for EvalError<'src> {
    fn from(err: TryFromValueError<'src>) -> Self {
        Self::WrongType(err)
    }
}

#[derive(Clone, Default)]
pub enum Value<'src> {
    #[default]
    Unit,
    Bool(bool),
    Int(i32),
    Path(LazyRc<'src, Path>),
    String(LazyRc<'src, str>),
    List(Rc<List<'src>>),
    Fn(LazyRc<'src, dyn ClispFn<'src> + 'src>),
}
macro_rules! impl_value_variant {
    ($variant:ident($ty:ty)) => {
        impl<'src> From<$ty> for Value<'src> {
            fn from(val: $ty) -> Self {
                Self::$variant(val)
            }
        }

        impl<'src> TryFromValue<'src> for $ty {
            fn try_from_value(val: Value<'src>) -> Result<$ty, TryFromValueError<'src>> {
                match val {
                    Value::$variant(val) => Ok(val),
                    val => Err(TryFromValueError(val, type_name::<$ty>())),
                }
            }
        }
    };
}
impl_value_variant!(Bool(bool));
impl_value_variant!(Int(i32));
impl_value_variant!(Path(LazyRc<'src, Path>));
impl_value_variant!(String(LazyRc<'src, str>));
impl_value_variant!(List(Rc<List<'src>>));
impl_value_variant!(Fn(LazyRc<'src, dyn ClispFn<'src> + 'src>));
impl Debug for Value<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        match self {
            Self::Unit => f.debug_tuple("Unit").finish(),
            Self::Bool(b) => f.debug_tuple("Bool").field(b).finish(),
            Self::Int(i) => f.debug_tuple("Int").field(i).finish(),
            Self::Path(p) => f.debug_tuple("Path").field(p).finish(),
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
            (Self::Path(l), Self::Path(r)) => l == r,
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
            Value::String(name) if name.as_ref() == "none" => Ok(None),
            Value::String(name) => Color::from_str(name.as_ref())
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
    UnknownColor(LazyRc<'src, str>),
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

pub trait TryFromValue<'src> {
    fn try_from_value(_: Value<'src>) -> Result<Self, TryFromValueError<'src>>
    where
        Self: Sized;
}
impl<'src, T> TryFromValue<'src> for T
where
    T: From<Value<'src>>,
{
    fn try_from_value(val: Value<'src>) -> Result<Self, TryFromValueError<'src>>
    where
        Self: Sized,
    {
        Ok(T::from(val))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum List<'src> {
    Nil,
    Cons(Value<'src>, Rc<Self>),
}
impl<'src> List<'src> {
    pub fn new<C, I>(iter: C) -> Rc<List<'src>>
    where
        C: IntoIterator<IntoIter = I, Item = Value<'src>>,
        I: DoubleEndedIterator + Iterator<Item = Value<'src>>,
    {
        iter.into_iter()
            .rev()
            .fold(Rc::new(List::Nil), |cdr, val| Rc::new(List::Cons(val, cdr)))
    }

    pub fn iter(self: Rc<Self>) -> list::Iter<'src> {
        list::Iter(self)
    }
}

#[derive(Debug)]
pub struct TryFromValueError<'src>(Value<'src>, &'static str);
impl<'src> Display for TryFromValueError<'src> {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "`{:?}` is not of type `{}`", self.0, self.1)
    }
}
