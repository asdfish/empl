use {
    crate::{
        config::clisp::{
            evaluator::{Arity, ClispFn, Environment, EvalError, Expr, List, TryFromValue, Value},
            lexer::Literal,
        },
        either::EitherOrBoth,
        ext::{array::ArrayExt, iterator::IteratorExt},
    },
    nonempty_collections::iter::IntoIteratorExt,
    std::{
        borrow::Cow,
        collections::{HashMap, HashSet, VecDeque},
        ops::Not,
        rc::Rc,
    },
};

fn eval_body<'src, I>(env: &mut Environment<'src>, iter: I) -> Result<Value<'src>, EvalError<'src>>
where
    I: IntoIterator<Item = Expr<'src>>,
{
    iter.try_into_nonempty_iter()
        .ok_or(EvalError::NoBody)?
        .into_iter()
        .map(|expr| env.eval(expr).map(Cow::into_owned))
        .try_fold(None, |_, expr| Ok(Some(expr?)))
        .transpose()
        .expect("should always have a value since the iterator is not empty")
}

fn cons<'src>(
    env: &mut Environment<'src>,
    args: VecDeque<Expr<'src>>,
) -> Result<Value<'src>, EvalError<'src>> {
    let [car, cdr] = args
        .into_iter()
        .collect_array::<2>()
        .ok_or(EvalError::WrongArity(Arity::Static(2)))?
        .map(|expr| env.eval(expr).map(Cow::into_owned))
        .transpose()?;

    let cdr = Rc::<List>::try_from_value(cdr)?;

    Ok(Value::List(Rc::new(List::Cons(car, cdr))))
}
fn r#if<'src>(
    env: &mut Environment<'src>,
    args: VecDeque<Expr<'src>>,
) -> Result<Value<'src>, EvalError<'src>> {
    let mut args = args.into_iter();
    let predicate = args
        .next()
        .ok_or(EvalError::WrongArity(Arity::Range(2..3)))?;

    let then = args
        .next()
        .ok_or(EvalError::WrongArity(Arity::Range(2..3)))?;
    let otherwise = args.next();
    if args.next().is_some() {
        return Err(EvalError::WrongArity(Arity::Range(2..3)));
    }

    if env
        .eval(predicate)
        .map(Cow::into_owned)
        .and_then(|predicate| bool::try_from_value(predicate).map_err(EvalError::WrongType))?
    {
        env.eval(then).map(Cow::into_owned)
    } else if let Some(otherwise) = otherwise {
        env.eval(otherwise).map(Cow::into_owned)
    } else {
        Ok(Value::Unit)
    }
}
fn lambda<'src>(
    _: &mut Environment<'src>,
    mut args: VecDeque<Expr<'src>>,
) -> Result<Value<'src>, EvalError<'src>> {
    let Expr::List(bindings) = args
        .pop_front()
        .ok_or(EvalError::WrongArity(Arity::RangeFrom(2..)))?
    else {
        return Err(EvalError::NoBindings);
    };
    let bindings = bindings
        .into_iter()
        .map(|expr| {
            if let Expr::Literal(Literal::Ident(ident)) = expr {
                Ok(*ident)
            } else {
                Err(EvalError::NonIdentBinding(expr))
            }
        })
        .collect::<Result<Vec<&'src str>, _>>()?;
    bindings.iter().try_fold(
        HashSet::with_capacity(bindings.len()),
        |mut bindings, binding| {
            if bindings.insert(binding) {
                Ok(bindings)
            } else {
                Err(EvalError::MultipleBindings(binding))
            }
        },
    )?;
    let body = args;

    Ok(Value::Fn(Box::new(move |env, args| {
        args.into_iter()
            .zip_all(&bindings)
            .try_for_each(|arg| match arg {
                EitherOrBoth::Both(arg, binding) => {
                    let arg = env.eval(arg).map(Cow::into_owned)?;
                    env.last_mut().insert(binding, arg);
                    Ok(())
                }
                _ => Err(EvalError::WrongArity(Arity::Static(bindings.len()))),
            })?;

        eval_body(env, body.iter().cloned())
    })))
}
fn r#let<'src>(
    env: &mut Environment<'src>,
    mut args: VecDeque<Expr<'src>>,
) -> Result<Value<'src>, EvalError<'src>> {
    args.pop_front()
        .ok_or(EvalError::NoBindings)
        .and_then(|bindings| match bindings {
            Expr::List(bindings) if bindings.is_empty() => Err(EvalError::EmptyListBindings),
            Expr::List(bindings) => Ok(bindings),
            expr => Err(EvalError::NonListBindings(expr)),
        })?
        .into_iter()
        .try_for_each(|binding| -> Result<(), EvalError<'src>> {
            match binding {
                Expr::List(binding) => {
                    let [binding, value] = binding
                        .into_iter()
                        .collect_array::<2>()
                        .ok_or(EvalError::WrongBindingArity(Arity::Static(2)))?;
                    let Expr::Literal(Literal::Ident(binding)) = binding else {
                        return Err(EvalError::NonIdentBinding(binding));
                    };
                    let value = env.eval(value).map(Cow::into_owned)?;
                    env.last_mut().insert(binding, value);
                    Ok(())
                }
                expr => Err(EvalError::NonListBindings(expr)),
            }
        })?;

    let body = args;
    eval_body(env, body.iter().cloned())
}
fn list<'src>(
    env: &mut Environment<'src>,
    args: VecDeque<Expr<'src>>,
) -> Result<Value<'src>, EvalError<'src>> {
    args.into_iter()
        .rev()
        .try_fold(Rc::new(List::Nil), |accum, item| {
            Ok(Rc::new(List::Cons(
                env.eval(item).map(Cow::into_owned)?,
                accum,
            )))
        })
        .map(Value::List)
}
fn math_fn<'src, O>(mut op: O) -> impl ClispFn<'src>
where
    O: Clone + Fn(i32, i32) -> Option<i32>,
{
    move |env, args| {
        let mut args = args.into_iter();
        let fst = args
            .next()
            .ok_or(EvalError::WrongArity(Arity::RangeFrom(2..)))
            .and_then(|fst| env.eval(fst).map(Cow::into_owned))
            .and_then(|fst| i32::try_from_value(fst).map_err(EvalError::WrongType))?;
        let args = args
            .try_into_nonempty_iter()
            .ok_or(EvalError::WrongArity(Arity::RangeFrom(2..)))?;

        args.into_iter()
            .try_fold(fst, |accum, i| {
                env.eval(i)
                    .map(Cow::into_owned)
                    .and_then(|i| i32::try_from_value(i).map_err(EvalError::WrongType))
                    .and_then(|i| op(accum, i).ok_or(EvalError::Overflow))
            })
            .map(Value::Int)
    }
}
fn nil<'src>(
    _: &mut Environment<'src>,
    _: VecDeque<Expr<'src>>,
) -> Result<Value<'src>, EvalError<'src>> {
    Ok(Value::List(Rc::new(List::Nil)))
}
fn not<'src>(
    env: &mut Environment<'src>,
    args: VecDeque<Expr<'src>>,
) -> Result<Value<'src>, EvalError<'src>> {
    let [predicate] = args
        .into_iter()
        .collect_array()
        .ok_or(EvalError::WrongArity(Arity::Static(1)))?;

    env.eval(predicate)
        .map(Cow::into_owned)
        .and_then(|predicate| {
            bool::try_from_value(predicate)
                .map(bool::not)
                .map(Value::Bool)
                .map_err(EvalError::WrongType)
        })
}

pub fn new<'a>() -> HashMap<&'a str, Value<'a>> {
    HashMap::from_iter([
        ("+", Value::Fn(Box::new(math_fn(i32::checked_add)))),
        ("-", Value::Fn(Box::new(math_fn(i32::checked_sub)))),
        ("/", Value::Fn(Box::new(math_fn(i32::checked_div)))),
        ("*", Value::Fn(Box::new(math_fn(i32::checked_mul)))),
        ("%", Value::Fn(Box::new(math_fn(i32::checked_rem)))),
        ("!", Value::Fn(Box::new(not))),
        ("cons", Value::Fn(Box::new(cons))),
        ("if", Value::Fn(Box::new(r#if))),
        ("lambda", Value::Fn(Box::new(lambda))),
        ("let", Value::Fn(Box::new(r#let))),
        ("list", Value::Fn(Box::new(list))),
        ("nil", Value::Fn(Box::new(nil))),
    ])
}
