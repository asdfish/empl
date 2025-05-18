use {
    crate::config::clisp::evaluator::{Environment, Value},
    std::iter::FusedIterator,
};

pub trait ClispFn<'a> {
    fn call<C, I>(&self, _: &mut Environment<'a>, _: C) -> Result<Option<Value<'a>>, FnCallError>
    where
        C: IntoIterator<IntoIter = I, Item = Value<'a>>,
        I: FusedIterator + Iterator<Item = Value<'a>>;
}
macro_rules! impl_clisp_fn_for {
    () => {};
    ($car:ident) => {
        #[expect(non_camel_case_types)]
        impl<'a, $car> ClispFn<'a> for fn() -> $car
        where
            $car: Into<Value<'a>>,
        {
            fn call<C, I>(&self, _: &mut Environment<'a>, args: C) -> Result<Option<Value<'a>>, FnCallError>
            where
                C: IntoIterator<IntoIter = I, Item = Value<'a>>,
                I: FusedIterator + Iterator<Item = Value<'a>>
            {
                if args.into_iter().next().is_some() {
                    Err(FnCallError::WrongArity {
                        expected: 0,
                        found: 1,
                    })
                } else {
                    Ok(Some((self)().into()))
                }
            }
        }
    };
    ($car:ident, $($cdr:ident),* $(,)?) => {
        impl_clisp_fn_for!($($cdr),*);

        #[expect(non_camel_case_types)]
        #[allow(non_upper_case_globals)]
        impl<'a, $car, $($cdr),*> ClispFn<'a> for fn($($cdr),*) -> $car
        where
            $car: Into<Value<'a>>,
            $($cdr: From<Value<'a>>),*
        {
            fn call<C, I>(&self, _: &mut Environment<'a>, args: C) -> Result<Option<Value<'a>>, FnCallError>
            where
                C: IntoIterator<IntoIter = I, Item = Value<'a>>,
                I: FusedIterator + Iterator<Item = Value<'a>>
            {
                const ARITY: usize = const {
                    $(const $cdr: () = ();)*
                    const ARGS: &[()] = &[$($cdr),*];
                    ARGS.len()
                };

                let mut args = args.into_iter();
                let mut arity = 0;
                let mut arg = || {
                    let output = args.next().ok_or(FnCallError::WrongArity {
                        expected: ARITY,
                        found: arity,
                    });
                    arity += 1;
                    
                    output
                };

                $(let $cdr = arg()?;)*
                arity += 1;
                if args.next().is_some() {
                    return Err(FnCallError::WrongArity {
                        expected: ARITY,
                        found: arity,
                    });
                }
                let output = (self)($($cdr.into()),*);

                Ok(Some(output.into()))
            }
        }
    }
}
impl_clisp_fn_for![a, b, c, d, e, f, g, h, i, j, k, l];

#[derive(Debug)]
pub enum FnCallError {
    WrongArity { expected: usize, found: usize },
}
