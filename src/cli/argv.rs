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

//! Module containing the code for an iterator over c style `argc` and `argv` pairs.

use std::{
    ffi::{CStr, c_char, c_int},
    slice,
};

/// Iterator for c style `argc` and `argv`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(transparent)]
pub struct Argv<'a>(&'a [*const c_char]);
impl<'a> Argv<'a> {
    /// Create a new argv iterator.
    ///
    /// # Examples
    ///
    /// ```
    /// #![cfg_attr(not(test), no_main)]
    /// #
    /// # use empl::cli::argv::Argv;
    /// # use std::ffi::{c_char, c_int};
    /// #
    /// #[cfg(not(test), unsafe(no_mangle))]
    /// extern "C" fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    ///     unsafe { Argv::new(argc, argv) }
    ///         .for_each(|arg| println!("{arg}"));
    ///
    ///     0
    /// }
    /// ```
    ///
    /// # Safety
    ///
    /// This function is safe if you follow the following preconditions:
    ///  - data must be non-null, valid for reads for `argc * size_of::<c_char>()` many bytes, and it must be properly aligned.
    ///  - data must point to `argc` consecutive properly initialized values of type [c_char].
    ///  - The memory referenced by the returned slice must not be mutated for the duration of lifetime `'a`, except inside an [UnsafeCell][std::cell::UnsafeCell].
    ///  - The memory pointed to by `argv[i]` must contain a valid nul terminator at the end of the string.
    ///  - `argv[i]` must be valid for reads of bytes up to and including the nul terminator.
    ///  - The entire memory range of every `argv[i]` must be contained within a single allocated object!
    ///  - The memory referenced by the returned [CStr] must not be mutated for the duration of lifetime `'a`.
    ///  - The nul terminator must be within [isize::MAX] from `argv[i]`
    pub unsafe fn new(argc: c_int, argv: *const *const c_char) -> Self {
        usize::try_from(argc)
            .ok()
            .filter(|_| !argv.is_null())
            // SAFETY:
            // The unfulfilled preconditions are placed into the contract.
            //  - [ ] data must be non-null, valid for reads for `argc * size_of::<c_char>()` many bytes, and it must be properly aligned.
            //  - [X] The entire memory range of this slice must be contained within a single allocated object! Slices can never span across multiple allocated objects. See below for an example incorrectly not taking this into account.
            //  - [X] data must be non-null and aligned even for zero-length slices or slices of ZSTs. One reason for this is that enum layout optimizations may rely on references (including slices of any length) being aligned and non-null to distinguish them from other data. You can obtain a pointer that is usable as data for zero-length slices using [NonNull::dangling].
            //  - [ ] data must point to argc consecutive properly initialized values of type [c_char].
            //  - [ ] The memory referenced by the returned slice must not be mutated for the duration of lifetime `'a`, except inside an [UnsafeCell][std::cell::UnsafeCell].
            //  - [X] The total size `argc * size_of::<c_char>()` of the slice must be no larger than [isize::MAX], and adding that size to data must not “wrap around” the address space. See the safety documentation of pointer::offset.
            .map(|argc| unsafe { slice::from_raw_parts(argv, argc) })
            .map(Self)
            .unwrap_or_default()
    }
}
impl<'a> Iterator for Argv<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        match self.0 {
            [] => None,
            [car, cdr @ ..] => {
                self.0 = cdr;

                if car.is_null() {
                    self.next()
                } else {
                    Some(
                        // SAFETY: the preconditions are thrown into [Self::new]
                        unsafe { CStr::from_ptr(*car) }.to_bytes(),
                    )
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, std::ptr};

    #[test]
    fn argv_new() {
        let foo = c"foo";
        let bar = c"bar";
        let foo_bar = [foo.as_ptr(), bar.as_ptr()];

        [0, -1]
            .into_iter()
            .flat_map(|argc| [(argc, ptr::dangling()), (argc, ptr::null())])
            .map(|(argc, argv)| (argc, argv, Argv(&[])))
            .chain((0..=2).map(|len| {
                (
                    len,
                    foo_bar.as_ptr(),
                    Argv(&foo_bar[..usize::try_from(len).unwrap()]),
                )
            }))
            .for_each(|(argc, argv, output)| assert_eq!(unsafe { Argv::new(argc, argv) }, output));
    }
}
