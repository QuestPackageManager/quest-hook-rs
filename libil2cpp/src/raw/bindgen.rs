#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(missing_docs)]
#![allow(dead_code)]
#![allow(clippy::ptr_offset_with_cast)]
// disable all warnings about generated bindings
#![cfg_attr(feature = "bindgen", allow(warnings))]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
