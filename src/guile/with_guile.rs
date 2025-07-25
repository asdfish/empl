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
    crate::guile::{self, GUILE_MODE, GuileModeToggle, INIT, INIT_LOCK},
    std::{ffi::c_void, marker::PhantomData, sync::atomic},
};

struct WithGuile<F, O>
where
    F: FnOnce(&mut guile::Api) -> O,
{
    _marker: PhantomData<(F, O)>,
}
impl<F, O> GuileModeToggle<F, O> for WithGuile<F, O>
where
    F: FnOnce(&mut guile::Api) -> O,
{
    const GUILE_MODE_STATE: bool = true;
    const SCM_FN: unsafe extern "C" fn(
        _: Option<unsafe extern "C" fn(_: *mut c_void) -> *mut c_void>,
        *mut c_void,
    ) -> *mut c_void = guile::sys::scm_with_guile;

    fn execute(operation: F) -> O {
        operation(&mut guile::Api(()))
    }
}

pub fn with_guile<F, O>(operation: F) -> O
where
    F: FnOnce(&mut guile::Api) -> O,
{
    if GUILE_MODE.with(|mode| mode.load(atomic::Ordering::Acquire)) {
        operation(&mut guile::Api(()))
    } else {
        let _lock = INIT
            .with(|init| !init.load(atomic::Ordering::Acquire))
            .then(|| INIT_LOCK.lock());

        WithGuile::call(|api| {
            INIT.with(|init| init.store(true, atomic::Ordering::Release));
            operation(api)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg_attr(miri, ignore)]
    #[test]
    fn nesting() {
        assert!(with_guile(|_| with_guile(|_| true)));
    }

    #[cfg_attr(miri, ignore)]
    #[test]
    fn multi_threading() {
        let spawn = || std::thread::spawn(|| with_guile(|_| {}));
        [(); 2]
            .map(|_| spawn())
            .into_iter()
            .for_each(|thread| thread.join().unwrap());
    }
}
