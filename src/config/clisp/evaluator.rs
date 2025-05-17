use {
    crate::config::clisp::{
        ast::Expr,
        lexer::Literal,
    },
    std::{
        borrow::Cow,
        collections::HashMap,
        rc::Rc,
    },
};

#[derive(Clone, Debug)]
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
        self.0
            .iter()
            .rev()
            .find_map(|vars| vars.get(ident))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value<'a> {
    Bool(bool),
    Int(i32),
    String(Cow<'a, Cow<'a, str>>),
    List(Box<List<'a>>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum List<'a> {
    Nil,
    Cons(Value<'a>, Rc<Self>),
}
