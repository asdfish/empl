use {
    crate::guile::{self, GuileFn},
    std::ffi::c_int,
};

impl guile::Api {
    pub fn define_fn<F>(&self)
    where
        F: GuileFn,
    {
        unsafe {
            guile::sys::scm_c_define_gsubr(
                F::NAME.as_ptr(),
                F::REQUIRED as c_int,
                F::OPTIONAL as c_int,
                F::REST.into(),
                F::DRIVER,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            guile::{Scm, guile_fn},
            tests::ENV_VAR_LOCK,
        },
        std::sync::atomic::{self, AtomicBool},
    };

    #[cfg_attr(miri, ignore)]
    #[test]
    fn define_fn() {
        let _lock = ENV_VAR_LOCK.read();

        static EXECUTED: AtomicBool = AtomicBool::new(false);

        #[guile_fn]
        fn set_executed([x]: [Scm; 1], _: [Option<Scm>; 0]) -> Scm {
            EXECUTED.store(true, atomic::Ordering::Release);
            x
        }

        guile::with_guile(|api| {
            api.define_fn::<SetExecuted>();
            // TODO: implement and use a wrapper for scm_eval_string
            unsafe {
                guile::sys::scm_eval_string(guile::sys::scm_from_utf8_string(
                    c"(set-executed 1)".as_ptr(),
                ));
            }
        });

        assert!(EXECUTED.load(atomic::Ordering::Acquire));
    }
}
