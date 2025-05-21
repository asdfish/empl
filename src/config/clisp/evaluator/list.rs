use {
    crate::config::clisp::evaluator::{List, Value},
    std::rc::Rc,
};

#[derive(Clone, Debug)]
pub struct Iter<'src>(pub(super) Rc<List<'src>>);
impl<'src> Iterator for Iter<'src> {
    type Item = Value<'src>;

    fn next(&mut self) -> Option<Value<'src>> {
        if let List::Cons(car, cdr) = Rc::unwrap_or_clone(Rc::clone(&self.0)) {
            self.0 = cdr;
            Some(car)
        } else {
            None
        }
    }
}
