//! PHP FFI Bindings
//!
//! Low-level FFI bindings to PHP's embed SAPI.
//! These are the raw C function declarations.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

#[cfg(feature = "php-embed")]
pub mod bindings {
    include!(concat!(env!("OUT_DIR"), "/php_bindings.rs"));
}

/// Re-export bindgen symbols so existing call sites keep working
#[cfg(feature = "php-embed")]
pub use bindings::*;

