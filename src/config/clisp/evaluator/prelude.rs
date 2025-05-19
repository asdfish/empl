use {
    crate::{
        config::clisp::evaluator::{List, FnCallError, TryFromValue, Value},
        ext::iterator::IteratorExt,
    },
    std::{
        borrow::Cow,
        collections::HashMap,
        rc::Rc,
    },
};

fn cons<'a>(args: Vec<Value<'a>>) -> Result<Value<'a>, FnCallError<'a>> {
    let Some([car, cdr]) = args.into_iter().collect_array() else {
        return Err(FnCallError::WrongArity(2));
    };
    let cdr: Rc<List<'a>> = Rc::try_from_value(cdr)?;

    Ok(Value::List(Rc::new(List::Cons(car, cdr))))
}
fn list<'a>(args: Vec<Value<'a>>) -> Result<Value<'a>, FnCallError<'a>> {
    Ok(Value::List(args
        .into_iter()
        .rev()
        .fold(Rc::new(List::Nil), |accum, item| Rc::new(List::Cons(item, accum)))))
}
fn nil<'a>(_: Vec<Value<'a>>) -> Result<Value<'a>, FnCallError<'a>> {
    Ok(Value::List(Rc::new(List::Nil)))
}

pub fn new<'a>() -> HashMap<&'a str, Value<'a>> {
    HashMap::from_iter([
        ("cons", Value::Fn(Cow::Borrowed(&cons))),
        ("list", Value::Fn(Cow::Borrowed(&list))),
        ("nil", Value::Fn(Cow::Borrowed(&nil))),
    ])
}
