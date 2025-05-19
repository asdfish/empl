use {
    crate::{
        config::clisp::{
            evaluator::{ClispFn, Environment, List, Expr, EvalError, TryFromValue, Value},
            lexer::Literal,
        },
        cow::NonStaticCow,
        ext::{
            array::ArrayExt,
            iterator::IteratorExt,
        },
    },
    nonempty_collections::{
        iter::{IntoIteratorExt, NonEmptyIterator},
        vector::NEVec,
    },
    std::{
        borrow::Cow,
        collections::{HashMap, VecDeque},
        rc::Rc,
        vec,
    },
};

// fn closure<F>(f: F) -> Box<dyn ClispFn>
// where F: ClispFn + 'static {
//     Box::new(f)
// }

fn cons<'env, 'src>(env: &'env mut Environment<'src>, args: VecDeque<Expr<'src>>) -> Result<Value<'src>, EvalError<'src>> {
    let [car, cdr] = args.into_iter().collect_array::<2>().ok_or(EvalError::WrongArity(2))?
        .map(|expr| env.eval(expr).map(Cow::into_owned))
        .transpose()?;

    let cdr = Rc::<List>::try_from_value(cdr)?;

    Ok(Value::List(Rc::new(List::Cons(car, cdr))))
}
fn lambda<'env, 'src>(_: &'env mut Environment<'src>, mut args: VecDeque<Expr<'src>>) -> Result<Value<'src>, EvalError<'src>> {
    let Expr::List(bindings) = args.pop_front().ok_or(EvalError::WrongVariadicArity(2..))? else {
        return Err(EvalError::NoBindings);
    };
    let bindings = bindings.into_iter()
        .map(|expr| if let Expr::Literal(Literal::Ident(ident)) = expr {
            Ok(ident)
        } else {
            Err(EvalError::NonIdentBinding(expr))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let body = args.try_into_nonempty_iter().ok_or(EvalError::NoBody)?
        .collect::<NEVec<_>>();

    Ok(Value::Fn(NonStaticCow::Owned(Box::<dyn ClispFn>::new(move |_, _| {
        println!("{:?}", body);

        todo!()
    }))))
}
fn list<'env, 'src>(env: &'env mut Environment<'src>, args: VecDeque<Expr<'src>>) -> Result<Value<'src>, EvalError<'src>> {
    args
        .into_iter()
        .rev()
        .try_fold(Rc::new(List::Nil), |accum, item| Ok(Rc::new(List::Cons(env.eval(item).map(Cow::into_owned)?, accum))))
        .map(Value::List)
}
fn nil<'env, 'src>(_: &'env mut Environment<'src>, _: VecDeque<Expr<'src>>) -> Result<Value<'src>, EvalError<'src>> {
    Ok(Value::List(Rc::new(List::Nil)))
}

pub fn new<'a>() -> HashMap<&'a str, Value<'a>> {
    HashMap::from_iter([
        ("cons", Value::Fn(NonStaticCow::Borrowed(&cons))),
        ("lambda", Value::Fn(NonStaticCow::Borrowed(&lambda))),
        ("list", Value::Fn(NonStaticCow::Borrowed(&list))),
        ("nil", Value::Fn(NonStaticCow::Borrowed(&nil))),
    ])
    // Default::default()
}
