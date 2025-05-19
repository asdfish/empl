use {
    crate::{
        config::clisp::evaluator::{List, FnCallError, TryFromValue, Value},
        ext::iterator::IteratorExt,
    },
    std::{
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
fn nil<'a>(_: Vec<Value<'a>>) -> Result<Value<'a>, FnCallError<'a>> {
    Ok(Value::List(Rc::new(List::Nil)))
}

pub fn new<'a>() -> HashMap<&'a str, Value<'a>> {
    let mut hm = HashMap::new();
    hm.insert("cons", Value::Fn(Box::new(cons)));
    hm.insert("nil", Value::Fn(Box::new(nil)));

    hm
}
