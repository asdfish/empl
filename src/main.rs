#![cfg_attr(not(test), no_main)]

use std::ffi::{c_char, c_int};

#[cfg_attr(not(test), unsafe(no_mangle))]
extern "system" fn main(_: *const *const c_char, _: c_int) -> c_int {
    0
}
