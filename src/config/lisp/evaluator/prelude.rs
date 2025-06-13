use {
    crate::{
        config::lisp::{
            evaluator::{
                list, Arity, Environment, EvalError, Expr, ExprTy, LispFn, List, TryFromValue,
                Value,
            },
            lexer::Literal,
        },
        either::EitherOrBoth,
        ext::{
            array::ArrayExt,
            iterator::IteratorExt,
            pair::{BiFunctor, BiTranspose},
        },
        lazy_rc::LazyRc,
    },
    nonempty_collections::iter::{IntoIteratorExt, NonEmptyIterator},
    std::{
        collections::{vec_deque, HashMap, HashSet, VecDeque},
        env,
        ops::{ControlFlow, Not},
        path::{self, Path, PathBuf},
        rc::Rc,
    },
};

const fn math_fn<'src, O>(op: O) -> impl LispFn<'src>
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
) -> impl LispFn<'src>
where
    Extractor: Clone + Fn(Value<'src>) -> Result<ExtractorOutput, EvalError>,
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
) -> impl LispFn<'src>
where
    A: Clone + Fn() -> Arity,
    E: Clone
        + Fn(&mut Environment<'src>, &mut vec_deque::IntoIter<Expr<'src>>) -> Result<EO, EvalError>,
    F: Clone
        + Fn(
            &mut Environment<'src>,
            EO,
            LazyRc<'src, dyn LispFn<'src> + 'src>,
            list::Iter<'src>,
        ) -> Result<FO, EvalError>,
    FO: Into<Value<'src>>,
{
    move |env, args| {
        let mut args = args.into_iter();
        let map = args
            .next()
            .ok_or(EvalError::WrongArity(arity()))
            .and_then(|map| env.eval_into::<LazyRc<'src, dyn LispFn<'src>>>(map))?;
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
const fn typed_predicate_fn<'src, P, T>(predicate: P) -> impl LispFn<'src>
where
    P: Clone + Fn(&T) -> bool,
    T: TryFromValue<'src>,
{
    predicate_fn(
        |value| T::try_from_value(value).map_err(EvalError::WrongType),
        predicate,
    )
}
const fn value_fn<'src, F>(f: F) -> impl LispFn<'src>
where
    F: Clone
        + Fn(
            &mut dyn Iterator<Item = Result<Value<'src>, EvalError>>,
        ) -> Result<Value<'src>, EvalError>,
{
    move |env, args| f(&mut args.into_iter().map(|expr| env.eval(expr)))
}

fn concat<'src>(
    env: &mut Environment<'src>,
    args: VecDeque<Expr<'src>>,
) -> Result<Value<'src>, EvalError> {
    (const {
        value_fn(|vals| {
            vals.map(|val| {
                val.and_then(|val| LazyRc::<str>::try_from_value(val).map_err(EvalError::WrongType))
            })
            .try_into_nonempty_iter()
            .ok_or(EvalError::WrongArity(Arity::RangeFrom(2..)))
            .and_then(|vals| vals.next().map_snd(Ok).bi_transpose())
            .map(|cons| cons.map_fst(|car| String::from(car.as_ref())))
            .and_then(|(mut car, mut cdr)| {
                cdr.try_for_each(|tail| tail.map(|tail| car.push_str(tail.as_ref())))
                    .map(move |_| car)
            })
            .map(Rc::from)
            .map(LazyRc::Owned)
            .map(Value::String)
        })
    })(env, args)
}
const fn cons<'src>() -> impl LispFn<'src> {
    value_fn(|args| {
        args.fuse()
            .collect_array::<2>()
            .ok_or(EvalError::WrongArity(Arity::Static(2)))
            .and_then(<[Result<Value<'src>, EvalError>; 2]>::transpose)
            .and_then(|[car, cdr]| {
                Rc::<List>::try_from_value(cdr)
                    .map(move |cdr| Rc::new(List::Cons(car, cdr)))
                    .map_err(EvalError::WrongType)
            })
            .map(Value::List)
    })
}
const fn env<'src>() -> impl LispFn<'src> {
    value_fn(|args| {
        args.into_iter()
            .fuse()
            .collect_array::<1>()
            .ok_or(EvalError::WrongArity(Arity::Static(1)))
            .and_then(|[var]| var)
            .and_then(|var| LazyRc::<str>::try_from_value(var).map_err(EvalError::WrongType))
            .and_then(|var| {
                env::var(var.as_ref()).map_err(|err| EvalError::EnvVar(err, var.into_owned()))
            })
            .map(Rc::from)
            .map(LazyRc::Owned)
            .map(Value::String)
    })
}
fn r#if<'src>(
    env: &mut Environment<'src>,
    args: VecDeque<Expr<'src>>,
) -> Result<Value<'src>, EvalError> {
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
) -> Result<Value<'src>, EvalError> {
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
                Err(EvalError::NonIdentListBinding(ExprTy::from(expr)))
            }
        })
        .collect::<Result<Vec<&'src str>, _>>()?;
    bindings.iter().try_fold(
        HashSet::with_capacity(bindings.len()),
        |mut bindings, binding| {
            if bindings.insert(binding) {
                Ok(bindings)
            } else {
                Err(EvalError::MultipleBindings(binding.to_string()))
            }
        },
    )?;
    let body = args;
    if body.is_empty() {
        return Err(EvalError::NoBody);
    }

    Ok(Value::Fn(LazyRc::Owned(Rc::new(move |env, args| {
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
    }))))
}
fn r#let<'src>(
    env: &mut Environment<'src>,
    mut args: VecDeque<Expr<'src>>,
) -> Result<Value<'src>, EvalError> {
    args.pop_front()
        .ok_or(EvalError::NoBindings)
        .and_then(|bindings| match bindings {
            Expr::List(bindings) if bindings.is_empty() => Err(EvalError::EmptyListBindings),
            Expr::List(bindings) => Ok(bindings),
            expr => Err(EvalError::NonIdentListBinding(ExprTy::from(expr))),
        })?
        .into_iter()
        .try_for_each(|binding| -> Result<(), EvalError> {
            match binding {
                Expr::List(binding) => {
                    let [binding, value] = binding
                        .into_iter()
                        .collect_array::<2>()
                        .ok_or(EvalError::WrongBindingArity(Arity::Static(2)))?;
                    let Expr::Literal(Literal::Ident(binding)) = binding else {
                        return Err(EvalError::NonIdentListBinding(ExprTy::from(binding)));
                    };
                    let value = env.eval(value)?;
                    env.last_mut().insert(binding, value);
                    Ok(())
                }
                expr => Err(EvalError::NonIdentListBinding(ExprTy::from(expr))),
            }
        })?;

    let body = args;
    progn(env, body.iter().cloned())
}
fn list<'src>(
    env: &mut Environment<'src>,
    args: VecDeque<Expr<'src>>,
) -> Result<Value<'src>, EvalError> {
    args.into_iter()
        .rev()
        .try_fold(Rc::new(List::Nil), |accum, item| {
            Ok(Rc::new(List::Cons(env.eval(item)?, accum)))
        })
        .map(Value::List)
}
fn nil<'src>(_: &mut Environment<'src>, _: VecDeque<Expr<'src>>) -> Result<Value<'src>, EvalError> {
    Ok(Value::List(Rc::new(List::Nil)))
}
fn not<'src>(
    env: &mut Environment<'src>,
    args: VecDeque<Expr<'src>>,
) -> Result<Value<'src>, EvalError> {
    let [predicate] = args
        .into_iter()
        .collect_array()
        .ok_or(EvalError::WrongArity(Arity::Static(1)))?;

    env.eval_into::<bool>(predicate)
        .map(bool::not)
        .map(Value::Bool)
}
const fn path<'src>() -> impl LispFn<'src> {
    value_fn(|string| {
        string
            .fuse()
            .collect_array::<1>()
            .ok_or(EvalError::WrongArity(Arity::Static(1)))
            .and_then(|[string]| string)
            .and_then(|string| LazyRc::<str>::try_from_value(string).map_err(EvalError::WrongType))
            .map(|string| match string {
                LazyRc::Borrowed(path) => LazyRc::Borrowed(Path::new(path)),
                LazyRc::Owned(path) => LazyRc::Owned(Rc::from(PathBuf::from(path.as_ref()))),
            })
            .map(Value::Path)
    })
}
const fn path_children<'src>() -> impl LispFn<'src> {
    value_fn(|paths| {
        paths
            .fuse()
            .collect_array::<1>()
            .ok_or(EvalError::WrongArity(Arity::Static(1)))
            .and_then(|[path]| path)
            .and_then(|path| {
                LazyRc::<Path>::try_from_value(path)
                    .map_err(EvalError::WrongType)
                    .and_then(|path| {
                        let path_clone = LazyRc::clone(&path);
                        path.read_dir()
                            .map(move |dir| {
                                dir.map(move |dir_ent| {
                                    dir_ent
                                        .map(|dir_ent| dir_ent.path())
                                        .map(Rc::from)
                                        .map(LazyRc::Owned)
                                        .map(Value::Path)
                                        .map_err(|err| {
                                            EvalError::ReadPath(
                                                err,
                                                path_clone.clone().into_owned(),
                                            )
                                        })
                                })
                            })
                            .map_err(|err| EvalError::ReadPath(err, path.into_owned()))
                            .and_then(Iterator::collect::<Result<Vec<_>, EvalError>>)
                    })
            })
            .map(List::new)
            .map(Value::List)
    })
}
const fn path_name<'src>() -> impl LispFn<'src> {
    value_fn(|path| {
        path.fuse()
            .collect_array::<1>()
            .ok_or(EvalError::WrongArity(Arity::Static(1)))
            .and_then(|[path]| path)
            .and_then(|path| LazyRc::<Path>::try_from_value(path).map_err(EvalError::WrongType))
            .map(|path| match path {
                LazyRc::Borrowed(path) => {
                    LazyRc::from(path.file_name().unwrap_or_default().to_string_lossy())
                }
                LazyRc::Owned(path) => LazyRc::Owned(Rc::from(
                    path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned(),
                )),
            })
            .map(Value::String)
    })
}
fn path_separator<'src>(
    _: &mut Environment<'src>,
    _: VecDeque<Expr<'src>>,
) -> Result<Value<'src>, EvalError> {
    Ok(Value::String(LazyRc::Borrowed(path::MAIN_SEPARATOR_STR)))
}
fn progn<'src, C, I>(env: &mut Environment<'src>, iter: C) -> Result<Value<'src>, EvalError>
where
    C: IntoIterator<IntoIter = I, Item = Expr<'src>>,
    I: DoubleEndedIterator + Iterator<Item = Expr<'src>>,
{
    let mut iter = iter.into_iter();
    iter.next_back()
        .ok_or(EvalError::NoBody)
        .and_then(|tail| {
            iter.try_for_each(|expr| env.eval(expr).map(drop))
                .map(move |_| tail)
        })
        .and_then(|tail| env.eval_tail_call(tail))
}
const fn seq_filter<'src>() -> impl LispFn<'src> {
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
const fn seq_find<'src>() -> impl LispFn<'src> {
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
const fn seq_flat_map<'src>() -> impl LispFn<'src> {
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
const fn seq_fold<'src>() -> impl LispFn<'src> {
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
const fn seq_map<'src>() -> impl LispFn<'src> {
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
) -> Result<Value<'src>, EvalError> {
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
) -> Result<Value<'src>, EvalError> {
    let [success, failure] = args
        .into_iter()
        .collect_array::<2>()
        .ok_or(EvalError::WrongArity(Arity::Static(2)))
        .and_then(|args| {
            args.map(|arg| env.eval_into::<LazyRc<'src, dyn LispFn<'src>>>(arg))
                .transpose()
        })?;

    success(env, VecDeque::new()).or_else(move |_| failure(env, VecDeque::new()))
}

pub fn new<'a>() -> HashMap<&'a str, Value<'a>> {
    macro_rules! adapt_const {
        ($fn:expr) => {{
            fn temp<'src>(
                env: &mut Environment<'src>,
                args: VecDeque<Expr<'src>>,
            ) -> Result<Value<'src>, EvalError> {
                ($fn)(env, args)
            }
            temp
        }};
    }

    HashMap::from_iter([
        (
            "+",
            Value::Fn(LazyRc::Borrowed(&adapt_const!(
                const { math_fn(i32::checked_add) }
            ))),
        ),
        (
            "-",
            Value::Fn(LazyRc::Borrowed(&adapt_const!(
                const { math_fn(i32::checked_sub) }
            ))),
        ),
        (
            "/",
            Value::Fn(LazyRc::Borrowed(&adapt_const!(
                const { math_fn(i32::checked_div) }
            ))),
        ),
        (
            "*",
            Value::Fn(LazyRc::Borrowed(&adapt_const!(
                const { math_fn(i32::checked_mul) }
            ))),
        ),
        (
            "%",
            Value::Fn(LazyRc::Borrowed(&adapt_const!(
                const { math_fn(i32::checked_rem) }
            ))),
        ),
        ("concat", Value::Fn(LazyRc::Borrowed(&concat))),
        (
            "cons",
            Value::Fn(LazyRc::Borrowed(&adapt_const!(const { cons() }))),
        ),
        (
            "env",
            Value::Fn(LazyRc::Borrowed(&adapt_const!(const { env() }))),
        ),
        ("if", Value::Fn(LazyRc::Borrowed(&r#if))),
        ("lambda", Value::Fn(LazyRc::Borrowed(&lambda))),
        ("let", Value::Fn(LazyRc::Borrowed(&r#let))),
        ("list", Value::Fn(LazyRc::Borrowed(&list))),
        ("nil", Value::Fn(LazyRc::Borrowed(&nil))),
        ("not", Value::Fn(LazyRc::Borrowed(&not))),
        (
            "path",
            Value::Fn(LazyRc::Borrowed(&adapt_const!(const { path() }))),
        ),
        (
            "path-children",
            Value::Fn(LazyRc::Borrowed(&adapt_const!(const { path_children() }))),
        ),
        (
            "path-exists",
            Value::Fn(LazyRc::Borrowed(&adapt_const!(
                const { typed_predicate_fn::<_, LazyRc<'src, Path>>(|path| path.exists()) }
            ))),
        ),
        (
            "path-is-dir",
            Value::Fn(LazyRc::Borrowed(&adapt_const!(
                const { typed_predicate_fn::<_, LazyRc<'src, Path>>(|path| path.is_dir()) }
            ))),
        ),
        (
            "path-is-file",
            Value::Fn(LazyRc::Borrowed(&adapt_const!(
                const { typed_predicate_fn::<_, LazyRc<'src, Path>>(|path| path.is_file()) }
            ))),
        ),
        (
            "path-name",
            Value::Fn(LazyRc::Borrowed(&adapt_const!(const { path_name() }))),
        ),
        (
            "path-separator",
            Value::Fn(LazyRc::Borrowed(&path_separator)),
        ),
        ("progn", Value::Fn(LazyRc::Borrowed(&progn))),
        (
            "seq-filter",
            Value::Fn(LazyRc::Borrowed(&adapt_const!(const { seq_filter() }))),
        ),
        (
            "seq-find",
            Value::Fn(LazyRc::Borrowed(&adapt_const!(const { seq_find() }))),
        ),
        (
            "seq-flat-map",
            Value::Fn(LazyRc::Borrowed(&adapt_const!(const { seq_flat_map() }))),
        ),
        (
            "seq-fold",
            Value::Fn(LazyRc::Borrowed(&adapt_const!(const { seq_fold() }))),
        ),
        (
            "seq-map",
            Value::Fn(LazyRc::Borrowed(&adapt_const!(const { seq_map() }))),
        ),
        ("seq-rev", Value::Fn(LazyRc::Borrowed(&seq_rev))),
        ("try-catch", Value::Fn(LazyRc::Borrowed(&try_catch))),
    ])
}
