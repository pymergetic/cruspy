//! Crate-root macros (must live here for rust-analyzer resolution).

#[macro_export]
macro_rules! CRUSPY_REGISTER_METHOD {
    ($model:ident, $method:ident, $rust_fn:ident) => {
        ::paste::paste! {
            #[::ctor::ctor]
            fn [<__cruspy_rust_ctor_ $rust_fn>]() {
                $crate::cruspy_root::registry::register_rust_method(
                    $model::FQN,
                    stringify!($method),
                    $rust_fn as *mut std::ffi::c_void,
                );
            }
        }
    };
}
