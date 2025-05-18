use {
    crate::{
        config::clisp::evaluator::{Environment, TryFromValue, TryFromValueError, Value},
        ext::iterator::IteratorExt,
    },
    dyn_clone::DynClone,
    std::iter::FusedIterator,
};

pub trait Args<'a>: FusedIterator + Iterator<Item = Value<'a>> {}
impl<'a, T> Args<'a> for T where T: FusedIterator + Iterator<Item = Value<'a>> {}

pub trait ClispFn<'a>: DynClone {
    fn call(
        &self,
        _: &mut Environment<'a>,
        _: &mut dyn Args<'a>,
    ) -> Result<Option<Value<'a>>, FnCallError<'a>>;
}
dyn_clone::clone_trait_object!(ClispFn<'_>);
macro_rules! impl_clisp_fn_for {
    () => {};
    ($car:ident) => {
        #[expect(non_camel_case_types)]
        impl<'a, $car> ClispFn<'a> for fn() -> $car
        where
            $car: Into<Value<'a>>,
        {
            fn call(&self, _: &mut Environment<'a>, args: &mut dyn Args<'a>) -> Result<Option<Value<'a>>, FnCallError<'a>>
            {
                if args.into_iter().next().is_some() {
                    Err(FnCallError::WrongArity(0))
                } else {
                    Ok(Some((self)().into()))
                }
            }
        }
        impl<'a> ClispFn<'a> for fn()
        {
            fn call(&self, _: &mut Environment<'a>, args: &mut dyn Args<'a>) -> Result<Option<Value<'a>>, FnCallError<'a>>
            {
                if args.into_iter().next().is_some() {
                    Err(FnCallError::WrongArity(0))
                } else {
                    (self)();
                    Ok(None)
                }
            }
        }
    };
    ($car:ident, $($cdr:ident),+ $(,)?) => {
        impl_clisp_fn_for!($($cdr),*);

        #[expect(non_camel_case_types)]
        #[expect(non_upper_case_globals)]
        impl<'a, $car, $($cdr),*> ClispFn<'a> for fn($($cdr),*) -> $car
        where
        $car: Into<Value<'a>>,
        $($cdr: TryFromValue<'a>),*
        {
            fn call(&self, _: &mut Environment<'a>, args: &mut dyn Args<'a>) -> Result<Option<Value<'a>>, FnCallError<'a>>
            {
                const ARITY: usize = const {
                    $(const $cdr: () = ();)*
                    [$($cdr),*]
                        .len()
                };

                let [$($cdr),*] = args.collect_array::<ARITY>().ok_or(FnCallError::WrongArity(ARITY))?;
                Ok(Some((self)($(<$cdr>::try_from_value($cdr)?),*).into()))
            }
        }

        #[expect(non_camel_case_types)]
        #[expect(non_upper_case_globals)]
        impl<'a, $($cdr),*> ClispFn<'a> for fn($($cdr),*)
        where
        $($cdr: TryFromValue<'a>),*
        {
            fn call(&self, _: &mut Environment<'a>, args: &mut dyn Args<'a>) -> Result<Option<Value<'a>>, FnCallError<'a>>
            {
                const ARITY: usize = const {
                    $(const $cdr: () = ();)*
                    [$($cdr),*]
                        .len()
                };

                let [$($cdr),*] = args.collect_array::<ARITY>().ok_or(FnCallError::WrongArity(ARITY))?;
                (self)($(<$cdr>::try_from_value($cdr)?),*);

                Ok(None)
            }
        }
    }
}
impl_clisp_fn_for![a, b, c, d, e, f, g, h, i, j, k, l];

pub enum FnCallError<'a> {
    WrongArity(usize),
    WrongType(TryFromValueError<'a>),
}
impl<'a> From<TryFromValueError<'a>> for FnCallError<'a> 
{
    fn from(err: TryFromValueError<'a>) -> Self {
        Self::WrongType(err)
    }
}
