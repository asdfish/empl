use {
    crate::{
        config::clisp::{
            evaluator::{
                Arity, ClispFn, Environment, EvalError, Expr, List, TryFromValue, Value, list,
            },
            lexer::Literal,
        },
        either::EitherOrBoth,
        ext::{array::ArrayExt, iterator::IteratorExt},
    },
    nonempty_collections::iter::IntoIteratorExt,
    std::{
        borrow::Cow,
        collections::{HashMap, HashSet, VecDeque, vec_deque},
        ops::Not,
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

    if env.eval_into::<bool>(predicate)? {
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
    if body.is_empty() {
        return Err(EvalError::NoBody);
    }

    Ok(Value::Fn(Rc::new(move |env, args| {
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

        progn(env, body.iter().cloned())
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
    progn(env, body.iter().cloned())
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
fn math_fn<'src, O>(op: O) -> impl ClispFn<'src>
where
    O: Clone + Fn(i32, i32) -> Option<i32>,
{
    move |env, args| {
        let mut args = args.into_iter();
        let fst = args
            .next()
            .ok_or(EvalError::WrongArity(Arity::RangeFrom(2..)))
            .and_then(|fst| env.eval_into::<i32>(fst))?;
        let args = args
            .try_into_nonempty_iter()
            .ok_or(EvalError::WrongArity(Arity::RangeFrom(2..)))?;

        args.into_iter()
            .try_fold(fst, |accum, i| {
                env.eval_into::<i32>(i)
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

    env.eval_into::<bool>(predicate)
        .map(bool::not)
        .map(Value::Bool)
}
fn progn<'src, I>(env: &mut Environment<'src>, iter: I) -> Result<Value<'src>, EvalError<'src>>
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
fn seq_fn<'src, A, E, EO, F, FO>(arity: A, get_extra_args: E, morphism: F) -> impl ClispFn<'src>
where
    A: Clone + Fn() -> Arity,
    E: Clone + Fn(&mut Environment<'src>, &mut vec_deque::IntoIter<Expr<'src>>) -> Result<EO, EvalError<'src>>,
    F: Clone
        + Fn(
            &mut Environment<'src>,
            EO,
            Rc<dyn ClispFn<'src> + 'src>,
            list::Iter<'src>,
        ) -> Result<FO, EvalError<'src>>,
    FO: Into<Value<'src>>,
{
    move |env, args| {
        let mut args = args.into_iter();
        let map = args
            .next()
            .ok_or(EvalError::WrongArity(arity()))
            .and_then(|map| env.eval_into::<Rc<dyn ClispFn<'src>>>(map))?;
        let seq = args
            .next()
            .ok_or(EvalError::WrongArity(arity()))
            .and_then(|seq| env.eval_into::<Rc<List<'src>>>(seq))?;
        let extra_args = get_extra_args(env, &mut args)?;
        if args.next().is_some() {
            return Err(EvalError::WrongArity(arity()));
        }

        morphism(env, extra_args, map, seq.iter()).map(FO::into)
    }
}

pub fn new<'a>() -> HashMap<&'a str, Value<'a>> {
    HashMap::from_iter([
        ("+", Value::Fn(Rc::new(math_fn(i32::checked_add)))),
        ("-", Value::Fn(Rc::new(math_fn(i32::checked_sub)))),
        ("/", Value::Fn(Rc::new(math_fn(i32::checked_div)))),
        ("*", Value::Fn(Rc::new(math_fn(i32::checked_mul)))),
        ("%", Value::Fn(Rc::new(math_fn(i32::checked_rem)))),
        ("cons", Value::Fn(Rc::new(cons))),
        ("if", Value::Fn(Rc::new(r#if))),
        ("lambda", Value::Fn(Rc::new(lambda))),
        ("let", Value::Fn(Rc::new(r#let))),
        ("list", Value::Fn(Rc::new(list))),
        ("nil", Value::Fn(Rc::new(nil))),
        ("not", Value::Fn(Rc::new(not))),
        ("progn", Value::Fn(Rc::new(progn))),
        (
            "seq-filter",
            Value::Fn(Rc::new(seq_fn(
                || Arity::Static(2),
                |_, _| Ok(()),
                |env, _, predicate, items| {
                    let predicates = items
                        .clone()
                        .map(|item| {
                            predicate(env, VecDeque::from([Expr::Value(item)])).and_then(|filter| {
                                bool::try_from_value(filter).map_err(EvalError::WrongType)
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?;

                    Ok(Value::List(List::new(
                        items
                            .zip(predicates)
                            .filter(|(_, predicate)| *predicate)
                            .map(|(item, _)| item)
                            .collect::<Vec<_>>(),
                    )))
                },
            ))),
        ),
        (
            "seq-find",
            Value::Fn(Rc::new(seq_fn(
                || Arity::Static(2),
                |_, _| Ok(()),
                |env, _, predicate, mut items| {
                    items
                        .find_map(|item| {
                            match predicate(env, VecDeque::from([Expr::Value(item.clone())]))
                                .and_then(|predicate| {
                                    bool::try_from_value(predicate).map_err(EvalError::WrongType)
                                }) {
                                Ok(true) => Some(Ok(item)),
                                Ok(false) => None,
                                Err(err) => Some(Err(err)),
                            }
                        })
                        .unwrap_or(Ok(Value::Unit))
                },
            ))),
        ),
        (
            "seq-flat-map",
            Value::Fn(Rc::new(seq_fn(
                || Arity::Static(2),
                |_, _| Ok(()),
                |env, _, map, items| {
                    items
                        .map(|item| {
                            map(env, VecDeque::from([Expr::Value(item)])).and_then(|item| {
                                Rc::<List<'a>>::try_from_value(item).map_err(EvalError::WrongType)
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()
                        .map(|list| list.into_iter().flat_map(List::iter).collect::<Vec<_>>())
                        .map(List::new)
                        .map(Value::List)
                },
            ))),
        ),
        ("seq-fold", Value::Fn(Rc::new(seq_fn(|| Arity::Static(3), |env, args| {
            args.next()
                .ok_or(EvalError::WrongArity(Arity::Static(3)))
                .and_then(|expr| env.eval(expr).map(Cow::into_owned))
        }, |env, accum, fold, mut items| {
            items.try_fold(accum, |accum, item| {
                fold(env, VecDeque::from([accum, item].map(Expr::Value)))
            })
        })))),
        (
            "seq-map",
            Value::Fn(Rc::new(seq_fn(
                || Arity::Static(2),
                |_, _| Ok(()),
                |env, _, map, items| {
                    items
                        .map(|item| map(env, VecDeque::from([Expr::Value(item)])))
                        .collect::<Result<Vec<_>, _>>()
                        .map(List::new)
                        .map(Value::List)
                },
            ))),
        ),
    ])
}
