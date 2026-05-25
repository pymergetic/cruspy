//! Rust-side registry hooks (mirrors C++ ``CRUSPY_REGISTER_METHOD``).

use std::ffi::CString;
use std::os::raw::c_char;

const LANG_RUST: i32 = 1;

extern "C" {
    fn cruspy_register_rust_method(
        fqn: *const c_char,
        method: *const c_char,
        rust_fn: *mut std::ffi::c_void,
        preferred: i32,
    ) -> i32;
}

pub(crate) fn register_rust_method(fqn: &str, method: &str, rust_fn: *mut std::ffi::c_void) {
    let cfqn = CString::new(fqn).expect("fqn");
    let cmethod = CString::new(method).expect("method");
    unsafe {
        cruspy_register_rust_method(cfqn.as_ptr(), cmethod.as_ptr(), rust_fn, LANG_RUST);
    }
}
