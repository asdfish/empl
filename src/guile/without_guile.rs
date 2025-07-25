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
    crate::guile::{self, GuileModeToggle},
    std::{ffi::c_void, marker::PhantomData},
};

struct WithoutGuile<F, O>
where
    F: FnOnce() -> O,
{
    _marker: PhantomData<(F, O)>,
}
impl<F, O> GuileModeToggle<F, O> for WithoutGuile<F, O>
where
    F: FnOnce() -> O,
{
    const GUILE_MODE_STATE: bool = false;
    const SCM_FN: unsafe extern "C" fn(
        _: Option<unsafe extern "C" fn(_: *mut c_void) -> *mut c_void>,
        *mut c_void,
    ) -> *mut c_void = guile::sys::scm_without_guile;

    fn execute(operation: F) -> O {
        operation()
    }
}

impl guile::Api {
    pub fn without_guile<F, O>(&mut self, operation: F) -> O
    where
        F: FnOnce() -> O,
    {
        WithoutGuile::call(operation)
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::tests::ENV_VAR_LOCK};

    #[cfg_attr(miri, ignore)]
    #[test]
    fn nesting() {
        let _lock = ENV_VAR_LOCK.read();
        assert!(guile::with_guile(
            |api| api.without_guile(|| guile::with_guile(|_| true))
        ));
    }
}
