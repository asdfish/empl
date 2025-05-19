use {
    crate::{
        config::clisp::{
            evaluator::{Environment, EvalError, Expr, List, TryFromValue, Value},
            lexer::Literal,
        },
        either::EitherOrBoth,
        ext::{array::ArrayExt, iterator::IteratorExt},
    },
    nonempty_collections::{
        iter::{IntoIteratorExt, NonEmptyIterator},
        vector::NEVec,
    },
    std::{
        borrow::Cow,
        collections::{HashMap, HashSet, VecDeque},
        rc::Rc,
    },
};

fn cons<'src>(
    env: &mut Environment<'src>,
    args: VecDeque<Expr<'src>>,
) -> Result<Value<'src>, EvalError<'src>> {
    let [car, cdr] = args
        .into_iter()
        .collect_array::<2>()
        .ok_or(EvalError::WrongArity(2))?
        .map(|expr| env.eval(expr).map(Cow::into_owned))
        .transpose()?;

    let cdr = Rc::<List>::try_from_value(cdr)?;

    Ok(Value::List(Rc::new(List::Cons(car, cdr))))
}
fn lambda<'src>(
    _: &mut Environment<'src>,
    mut args: VecDeque<Expr<'src>>,
) -> Result<Value<'src>, EvalError<'src>> {
    let Expr::List(bindings) = args.pop_front().ok_or(EvalError::WrongVariadicArity(2..))? else {
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
    let body = args
        .try_into_nonempty_iter()
        .ok_or(EvalError::NoBody)?
        .collect::<NEVec<_>>();

    Ok(Value::Fn(Box::new(move |env, args| {
        args.into_iter()
            .zip_all(&bindings)
            .try_for_each(|arg| match arg {
                EitherOrBoth::Both(arg, binding) => {
                    let arg = env.eval(arg).map(Cow::into_owned)?;
                    if env.last_mut().insert(binding, arg).is_none() {
                        Ok(())
                    } else {
                        Err(EvalError::MultipleBindings(binding))
                    }
                }
                _ => Err(EvalError::WrongArity(bindings.len())),
            })?;

        body.iter()
            .cloned()
            .map(|expr| env.eval(expr).map(Cow::into_owned))
            .try_fold(None, |_, expr| Ok(Some(expr?)))
            .transpose()
            .expect("should always have a value since the iterator is not empty")
    })))
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
fn nil<'src>(
    _: &mut Environment<'src>,
    _: VecDeque<Expr<'src>>,
) -> Result<Value<'src>, EvalError<'src>> {
    Ok(Value::List(Rc::new(List::Nil)))
}

pub fn new<'a>() -> HashMap<&'a str, Value<'a>> {
    HashMap::from_iter([
        ("cons", Value::Fn(Box::new(cons))),
        ("lambda", Value::Fn(Box::new(lambda))),
        ("list", Value::Fn(Box::new(list))),
        ("nil", Value::Fn(Box::new(nil))),
    ])
}
