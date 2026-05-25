//! Hand-written Hello model — Rust method bodies (EP-0021).

include!("hello_gen.rs");

use crate::CRUSPY_REGISTER_RUST_METHOD;
use crate::cruspy_root::runtime::kernel::{field_get_string, MemoryHandle as KernelMemoryHandle};

#[no_mangle]
pub unsafe extern "C" fn hello_rust(
    handle: *const KernelMemoryHandle,
    out: *mut u8,
    capacity: usize,
) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let message = match field_get_string(&*handle, "message") {
        Ok(value) => value,
        Err(err) => return err.0,
    };
    let greeting = format!("Hello from Rust — {message}");
    if capacity == 0 {
        return greeting.len() as i32;
    }
    if out.is_null() || capacity < greeting.len() {
        return -1;
    }
    std::ptr::copy_nonoverlapping(greeting.as_ptr(), out, greeting.len());
    greeting.len() as i32
}

CRUSPY_REGISTER_RUST_METHOD!(FQN, "hello_rust", hello_rust);
