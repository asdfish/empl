use {
    crate::{
        config::clisp::{
            evaluator::{
                Arity, ClispFn, Environment, EvalError, Expr, List, TryFromValue, Value, list,
            },
            lexer::Literal,
        },
        either::EitherOrBoth,
        ext::{array::ArrayExt, iterator::IteratorExt, pair::PairExt},
    },
    nonempty_collections::iter::{IntoIteratorExt, NonEmptyIterator},
    std::{
        collections::{HashMap, HashSet, VecDeque, vec_deque},
        env,
        ops::{ControlFlow, Not},
        path::{Path, PathBuf},
        rc::Rc,
    },
    supercow::Supercow,
};

const fn math_fn<'src, O>(op: O) -> impl ClispFn<'src>
where
    O: Clone + Fn(i32, i32) -> Option<i32>,
{
    value_fn(move |args| {
        let fst = args
            .next()
            .ok_or(EvalError::WrongArity(Arity::RangeFrom(2..)))?
            .and_then(|fst| i32::try_from_value(fst).map_err(EvalError::WrongType))?;

        args.try_into_nonempty_iter()
            .ok_or(EvalError::WrongArity(Arity::RangeFrom(2..)))?
            .into_iter()
            .try_fold(fst, |accum, operand| {
                operand
                    .and_then(|operand| i32::try_from_value(operand).map_err(EvalError::WrongType))
                    .and_then(|operand| op(accum, operand).ok_or(EvalError::Overflow))
            })
            .map(Value::Int)
    })
}
const fn predicate_fn<'src, Extractor, ExtractorOutput, Predicate>(
    extractor: Extractor,
    predicate: Predicate,
) -> impl ClispFn<'src>
where
    Extractor: Clone + Fn(Value<'src>) -> Result<ExtractorOutput, EvalError<'src>>,
    Predicate: Clone + Fn(&ExtractorOutput) -> bool,
{
    value_fn(move |args| {
        args.try_into_nonempty_iter()
            .ok_or(EvalError::WrongArity(Arity::RangeFrom(1..)))
            .and_then(|args| {
                match args
                    .into_iter()
                    .try_fold(true, |_, input| {
                        input
                            .map(
                                |input| match extractor(input).map(|input| predicate(&input)) {
                                    Ok(true) => ControlFlow::Continue(true),
                                    Ok(false) => ControlFlow::Break(Ok(false)),
                                    Err(err) => ControlFlow::Break(Err(err)),
                                },
                            )
                            .map_err(Err)
                            .unwrap_or_else(ControlFlow::Break)
                    })
                    .map_continue(Ok)
                {
                    ControlFlow::Continue(output) | ControlFlow::Break(output) => output,
                }
                .map(Value::Bool)
            })
    })
}
const fn seq_fn<'src, A, E, EO, F, FO>(
    arity: A,
    get_extra_args: E,
    morphism: F,
) -> impl ClispFn<'src>
where
    A: Clone + Fn() -> Arity,
    E: Clone
        + Fn(
            &mut Environment<'src>,
            &mut vec_deque::IntoIter<Expr<'src>>,
        ) -> Result<EO, EvalError<'src>>,
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
const fn value_fn<'src, F>(f: F) -> impl ClispFn<'src>
where
    F: Clone
        + Fn(
            &mut dyn Iterator<Item = Result<Value<'src>, EvalError<'src>>>,
        ) -> Result<Value<'src>, EvalError<'src>>,
{
    move |env, args| f(&mut args.into_iter().map(|expr| env.eval(expr)))
}

const fn concat<'src>() -> impl ClispFn<'src> {
    value_fn(|vals| {
        let (mut car, mut cdr) = vals
            .map(|val| {
                val.and_then(|val| {
                    Supercow::<'src, String, str, Rc<str>>::try_from_value(val)
                        .map_err(EvalError::WrongType)
                })
            })
            .try_into_nonempty_iter()
            .ok_or(EvalError::WrongArity(Arity::RangeFrom(2..)))?
            .next()
            .transpose_fst()?;
        cdr.try_for_each(|item| item.map(|item| car.to_mut().push_str(item.as_ref())))?;

        Ok(Value::String(car))
    })
}
const fn cons<'src>() -> impl ClispFn<'src> {
    value_fn(|args| {
        args.fuse()
            .collect_array::<2>()
            .ok_or(EvalError::WrongArity(Arity::Static(2)))
            .and_then(<[Result<Value<'src>, EvalError<'src>>; 2]>::transpose)
            .and_then(|[car, cdr]| {
                Rc::<List>::try_from_value(cdr)
                    .map(move |cdr| Rc::new(List::Cons(car, cdr)))
                    .map_err(EvalError::WrongType)
            })
            .map(Value::List)
    })
}
const fn env<'src>() -> impl ClispFn<'src> {
    value_fn(|args| {
        args.into_iter()
            .fuse()
            .collect_array::<1>()
            .ok_or(EvalError::WrongArity(Arity::Static(1)))
            .and_then(|[var]| var)
            .and_then(|var| {
                Supercow::<'src, String, str, Rc<str>>::try_from_value(var)
                    .map_err(EvalError::WrongType)
            })
            .and_then(|var| env::var(var.as_ref()).map_err(EvalError::EnvVar))
            .map(Supercow::owned)
            .map(Value::String)
    })
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
        env.eval(then)
    } else if let Some(otherwise) = otherwise {
        env.eval(otherwise)
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
                    let arg = env.eval(arg)?;
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
                    let value = env.eval(value)?;
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
            Ok(Rc::new(List::Cons(env.eval(item)?, accum)))
        })
        .map(Value::List)
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
const fn path<'src>() -> impl ClispFn<'src> {
    value_fn(|string| {
        string
            .fuse()
            .collect_array::<1>()
            .ok_or(EvalError::WrongArity(Arity::Static(1)))
            .and_then(|[string]| string)
            .and_then(|string| {
                Supercow::<'src, String, str, Rc<str>>::try_from_value(string)
                    .map_err(EvalError::WrongType)
            })
            .map(|string| {
                Supercow::extract_ref(&string)
                    .map(Path::new)
                    .map(Supercow::borrowed)
                    .unwrap_or(Supercow::owned(PathBuf::from(Supercow::into_inner(string))))
            })
            .map(Value::Path)
    })
}
const fn path_children<'src>() -> impl ClispFn<'src> {
    value_fn(|paths| {
        paths
            .fuse()
            .collect_array::<1>()
            .ok_or(EvalError::WrongArity(Arity::Static(1)))
            .and_then(|[path]| path)
            .and_then(|path| {
                Supercow::<'src, PathBuf, Path, Rc<Path>>::try_from_value(path)
                    .map_err(EvalError::WrongType)
                    .and_then(|path| {
                        path.read_dir()
                            .map(|dir| {
                                dir.map(|dir_ent| {
                                    dir_ent
                                        .map(|dir_ent| dir_ent.path())
                                        .map(Supercow::owned)
                                        .map(Value::Path)
                                        .map_err(EvalError::Io)
                                })
                            })
                            .map_err(EvalError::Io)
                            .and_then(Iterator::collect::<Result<Vec<_>, _>>)
                    })
            })
            .map(List::new)
            .map(Value::List)
    })
}
const fn path_exists<'src>() -> impl ClispFn<'src> {
    predicate_fn(
        |val| {
            Supercow::<'src, PathBuf, Path, Rc<Path>>::try_from_value(val)
                .map_err(EvalError::WrongType)
        },
        |path| path.exists(),
    )
}
const fn path_is_dir<'src>() -> impl ClispFn<'src> {
    predicate_fn(
        |val| {
            Supercow::<'src, PathBuf, Path, Rc<Path>>::try_from_value(val)
                .map_err(EvalError::WrongType)
        },
        |path| path.is_dir(),
    )
}
const fn path_is_file<'src>() -> impl ClispFn<'src> {
    predicate_fn(
        |val| {
            Supercow::<'src, PathBuf, Path, Rc<Path>>::try_from_value(val)
                .map_err(EvalError::WrongType)
        },
        |path| path.is_file(),
    )
}
const fn path_name<'src>() -> impl ClispFn<'src> {
    value_fn(|path| {
        path.fuse()
            .collect_array::<1>()
            .ok_or(EvalError::WrongArity(Arity::Static(1)))
            .and_then(|[path]| path)
            .and_then(|path| {
                Supercow::<'src, PathBuf, Path, Rc<Path>>::try_from_value(path)
                    .map_err(EvalError::WrongType)
            })
            .map(|path| {
                path.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned()
            })
            .map(Supercow::owned)
            .map(Value::String)
    })
}
fn progn<'src, I>(env: &mut Environment<'src>, iter: I) -> Result<Value<'src>, EvalError<'src>>
where
    I: IntoIterator<Item = Expr<'src>>,
{
    iter.try_into_nonempty_iter()
        .ok_or(EvalError::NoBody)?
        .into_iter()
        .map(|expr| env.eval(expr))
        .try_fold(None, |_, expr| Ok(Some(expr?)))
        .transpose()
        .expect("should always have a value since the iterator is not empty")
}
const fn seq_filter<'src>() -> impl ClispFn<'src> {
    seq_fn(
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
    )
}
const fn seq_find<'src>() -> impl ClispFn<'src> {
    seq_fn(
        || Arity::Static(2),
        |_, _| Ok(()),
        |env, _, predicate, mut items| {
            items
                .find_map(|item| {
                    match predicate(env, VecDeque::from([Expr::Value(item.clone())])).and_then(
                        |predicate| bool::try_from_value(predicate).map_err(EvalError::WrongType),
                    ) {
                        Ok(true) => Some(Ok(item)),
                        Ok(false) => None,
                        Err(err) => Some(Err(err)),
                    }
                })
                .unwrap_or(Ok(Value::Unit))
        },
    )
}
const fn seq_flat_map<'src>() -> impl ClispFn<'src> {
    seq_fn(
        || Arity::Static(2),
        |_, _| Ok(()),
        |env, _, map, items| {
            items
                .map(|item| {
                    map(env, VecDeque::from([Expr::Value(item)])).and_then(|item| {
                        Rc::<List<'src>>::try_from_value(item).map_err(EvalError::WrongType)
                    })
                })
                .collect::<Result<Vec<_>, _>>()
                .map(|list| list.into_iter().flat_map(List::iter).collect::<Vec<_>>())
                .map(List::new)
                .map(Value::List)
        },
    )
}
const fn seq_fold<'src>() -> impl ClispFn<'src> {
    seq_fn(
        || Arity::Static(3),
        |env, args| {
            args.next()
                .ok_or(EvalError::WrongArity(Arity::Static(3)))
                .and_then(|expr| env.eval(expr))
        },
        |env, accum, fold, mut items| {
            items.try_fold(accum, |accum, item| {
                fold(env, VecDeque::from([accum, item].map(Expr::Value)))
            })
        },
    )
}
const fn seq_map<'src>() -> impl ClispFn<'src> {
    seq_fn(
        || Arity::Static(2),
        |_, _| Ok(()),
        |env, _, map, items| {
            items
                .map(|item| map(env, VecDeque::from([Expr::Value(item)])))
                .collect::<Result<Vec<_>, _>>()
                .map(List::new)
                .map(Value::List)
        },
    )
}
fn seq_rev<'src>(
    env: &mut Environment<'src>,
    args: VecDeque<Expr<'src>>,
) -> Result<Value<'src>, EvalError<'src>> {
    args.into_iter()
        .collect_array()
        .ok_or(EvalError::WrongArity(Arity::Static(1)))
        .and_then(|[seq]| env.eval_into::<Rc<List<'src>>>(seq))
        .map(|seq| {
            seq.iter()
                .fold(Rc::new(List::Nil), |cdr, car| Rc::new(List::Cons(car, cdr)))
        })
        .map(Value::List)
}
fn try_catch<'src>(
    env: &mut Environment<'src>,
    args: VecDeque<Expr<'src>>,
) -> Result<Value<'src>, EvalError<'src>> {
    let [success, failure] = args
        .into_iter()
        .collect_array::<2>()
        .ok_or(EvalError::WrongArity(Arity::Static(2)))
        .and_then(|args| {
            args.map(|arg| env.eval_into::<Rc<dyn ClispFn<'src>>>(arg))
                .transpose()
        })?;

    success(env, VecDeque::new()).or_else(move |_| failure(env, VecDeque::new()))
}

pub fn new<'a>() -> HashMap<&'a str, Value<'a>> {
    HashMap::from_iter([
        ("+", Value::Fn(Rc::new(const { math_fn(i32::checked_add) }))),
        ("-", Value::Fn(Rc::new(const { math_fn(i32::checked_sub) }))),
        ("/", Value::Fn(Rc::new(const { math_fn(i32::checked_div) }))),
        ("*", Value::Fn(Rc::new(const { math_fn(i32::checked_mul) }))),
        ("%", Value::Fn(Rc::new(const { math_fn(i32::checked_rem) }))),
        ("concat", Value::Fn(Rc::new(const { concat() }))),
        ("cons", Value::Fn(Rc::new(const { cons() }))),
        ("env", Value::Fn(Rc::new(const { env() }))),
        ("if", Value::Fn(Rc::new(r#if))),
        ("lambda", Value::Fn(Rc::new(lambda))),
        ("let", Value::Fn(Rc::new(r#let))),
        ("list", Value::Fn(Rc::new(list))),
        ("nil", Value::Fn(Rc::new(nil))),
        ("not", Value::Fn(Rc::new(not))),
        ("path", Value::Fn(Rc::new(const { path() }))),
        (
            "path-children",
            Value::Fn(Rc::new(const { path_children() })),
        ),
        ("path-exists", Value::Fn(Rc::new(const { path_exists() }))),
        ("path-is-dir", Value::Fn(Rc::new(const { path_is_dir() }))),
        ("path-is-file", Value::Fn(Rc::new(const { path_is_file() }))),
        ("path-name", Value::Fn(Rc::new(const { path_name() }))),
        ("progn", Value::Fn(Rc::new(progn))),
        ("seq-filter", Value::Fn(Rc::new(const { seq_filter() }))),
        ("seq-find", Value::Fn(Rc::new(const { seq_find() }))),
        ("seq-flat-map", Value::Fn(Rc::new(const { seq_flat_map() }))),
        ("seq-fold", Value::Fn(Rc::new(const { seq_fold() }))),
        ("seq-map", Value::Fn(Rc::new(const { seq_map() }))),
        ("seq-rev", Value::Fn(Rc::new(seq_rev))),
        ("try-catch", Value::Fn(Rc::new(try_catch))),
    ])
}
