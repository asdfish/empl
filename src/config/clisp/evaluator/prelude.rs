use {
    crate::{
        config::clisp::evaluator::{Environment, List, Expr, EvalError, TryFromValue, Value},
        ext::{
            array::ArrayExt,
            iterator::IteratorExt,
        },
    },
    std::{
        borrow::Cow,
        collections::HashMap,
        rc::Rc,
        vec,
    },
};

fn cons<'env, 'src>(env: &'env mut Environment<'src>, args: vec::IntoIter<Expr<'src>>) -> Result<Value<'src>, EvalError<'src>> {
    let [car, cdr] = args.collect_array::<2>().ok_or(EvalError::WrongArity(2))?
        .map(|expr| env.eval(expr).map(Cow::into_owned))
        .transpose()?;

    let cdr = Rc::<List>::try_from_value(cdr)?;

    Ok(Value::List(Rc::new(List::Cons(car, cdr))))
}
fn list<'env, 'src>(env: &'env mut Environment<'src>, args: vec::IntoIter<Expr<'src>>) -> Result<Value<'src>, EvalError<'src>> {
    args
        .into_iter()
        .rev()
        .try_fold(Rc::new(List::Nil), |accum, item| Ok(Rc::new(List::Cons(env.eval(item).map(Cow::into_owned)?, accum))))
        .map(Value::List)
}
fn nil<'env, 'src>(_: &'env mut Environment<'src>, _: vec::IntoIter<Expr<'src>>) -> Result<Value<'src>, EvalError<'src>> {
    Ok(Value::List(Rc::new(List::Nil)))
}

pub fn new<'a>() -> HashMap<&'a str, Value<'a>> {
    HashMap::from_iter([
        ("cons", Value::Fn(Cow::Borrowed(&cons))),
        ("list", Value::Fn(Cow::Borrowed(&list))),
        ("nil", Value::Fn(Cow::Borrowed(&nil))),
    ])
}
