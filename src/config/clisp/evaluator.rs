pub mod function;

use {
    crate::config::clisp::{ast::Expr, lexer::Literal},
    dyn_clone::DynClone,
    std::{any::Any, borrow::Cow, collections::HashMap, rc::Rc},
};

#[derive(Clone)]
pub struct Environment<'a>(Vec<HashMap<&'a str, Value<'a>>>);
impl<'src> Environment<'src> {
    pub fn eval<'env>(&'env self, expr: Expr<'src>) -> Option<Cow<'env, Value<'src>>> {
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
impl<T> DynValue for T
where T: Any + DynClone {}
dyn_clone::clone_trait_object!(DynValue);

#[derive(Clone)]
pub enum Value<'a> {
    Bool(bool),
    Int(i32),
    String(Cow<'a, Cow<'a, str>>),
    List(Box<List<'a>>),
    Dyn(Box<dyn DynValue>),
}

#[derive(Clone)]
pub enum List<'a> {
    Nil,
    Cons(Value<'a>, Rc<Self>),
}
