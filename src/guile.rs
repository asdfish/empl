// empl - Extensible Music PLayer
// Copyright (C) 2025  Andrew Chi

// This file is part of empl.

// empl is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// empl is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with empl.  If not, see <http://www.gnu.org/licenses/>.

use {
    parking_lot::Mutex,
    std::{
        ffi::{CStr, c_void},
        ptr,
        sync::atomic::{self, AtomicBool},
    },
};

mod define_fn;
mod with_guile;
mod without_guile;

pub use {crate::guile::with_guile::with_guile, proc_macros::guile_fn};

/// Lock for thread initialization
static INIT_LOCK: Mutex<()> = const { Mutex::new(()) };

thread_local! {
    /// Whether the current thread has been initiated yet
    static INIT: AtomicBool = const { AtomicBool::new(false) };
    /// Whether the current thread is currently in guile mode.
    static GUILE_MODE: AtomicBool = const { AtomicBool::new(false) };
}

pub struct Api(());

struct GuileModeToggleData<F, O> {
    operation: Option<F>,
    output: Option<O>,
}
trait GuileModeToggle<F, O> {
    /// The state that [GUILE_MODE] should be in at the start of the call.
    const GUILE_MODE_STATE: bool;
    /// A pointer to the corresponding `scm_*` function.
    const SCM_FN: unsafe extern "C" fn(
        _: Option<unsafe extern "C" fn(_: *mut c_void) -> *mut c_void>,
        *mut c_void,
    ) -> *mut c_void;

    /// Evaluate `F` into `O`
    fn execute(_: F) -> O;

    /// # Safety
    ///
    /// `ptr` must be a pointer of type `GuileModeToggleData<F, O>`
    unsafe extern "C" fn callback(ptr: *mut c_void) -> *mut c_void {
        GUILE_MODE.with(|mode| mode.store(Self::GUILE_MODE_STATE, atomic::Ordering::Release));

        let data = ptr.cast::<GuileModeToggleData<F, O>>();
        if let Some(data) = unsafe { data.as_mut() }
            && let Some(operation) = data.operation.take()
            && data.output.is_none()
        {
            data.output = Some(Self::execute(operation));
        }

        ptr::null_mut()
    }

    fn call(operation: F) -> O {
        let mut data = GuileModeToggleData {
            operation: Some(operation),
            output: None,
        };

        unsafe {
            (Self::SCM_FN)(Some(Self::callback), (&raw mut data).cast());
        }
        GUILE_MODE.with(|mode| mode.store(!Self::GUILE_MODE_STATE, atomic::Ordering::Release));

        data.output.unwrap()
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Scm(sys::SCM);
impl Scm {
    pub const fn new(scm: sys::SCM) -> Self {
        Self(scm)
    }
}

pub trait GuileFn {
    const REQUIRED: usize;
    const OPTIONAL: usize;
    const REST: bool;

    const NAME: &CStr;
    const DRIVER: sys::scm_t_subr;
}

pub mod sys {
    #![allow(improper_ctypes)]
    #![expect(non_camel_case_types)]
    #![expect(non_snake_case)]
    #![expect(non_upper_case_globals)]

    include!(concat!(env!("OUT_DIR"), "/libguile.rs"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guile_fn_impl() {
        #[guile_fn]
        fn foo([input]: [Scm; 1], _: [Option<Scm>; 1]) -> Scm {
            input
        }
        assert_eq!(Foo::REQUIRED, 1);
        assert_eq!(Foo::OPTIONAL, 1);
        assert_eq!(Foo::REST, false);
        assert_eq!(Foo::NAME, c"foo");

        #[guile_fn]
        fn bar(_: [Scm; 0], _: [Option<Scm>; 1]) -> Scm {
            unimplemented!()
        }
        assert_eq!(Bar::REQUIRED, 0);
        assert_eq!(Bar::OPTIONAL, 1);
        assert_eq!(Bar::REST, false);
        assert_eq!(Bar::NAME, c"bar");

        #[guile_fn]
        fn baz(_: [Scm; 0], _: [Option<Scm>; 0], _: Scm) -> Scm {
            unimplemented!()
        }
        assert_eq!(Baz::REQUIRED, 0);
        assert_eq!(Baz::OPTIONAL, 0);
        assert_eq!(Baz::REST, true);
        assert_eq!(Baz::NAME, c"baz");
    }
}
