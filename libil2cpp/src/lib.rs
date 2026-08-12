#![feature(never_type)]
#![feature(trait_alias)]
#![feature(allocator_api)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]
#![doc(html_root_url = "https://stackdoubleflow.github.io/quest-hook-rs/libil2cpp")]
#![warn(
    clippy::all,
    clippy::await_holding_lock,
    clippy::char_lit_as_u8,
    clippy::checked_conversions,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_markdown,
    clippy::enum_glob_use,
    clippy::exit,
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_deref_methods,
    clippy::explicit_into_iter_loop,
    clippy::fallible_impl_from,
    clippy::filter_map_next,
    clippy::float_cmp_const,
    clippy::fn_params_excessive_bools,
    clippy::if_let_mutex,
    clippy::implicit_clone,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::invalid_upcast_comparisons,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::manual_ok_or,
    clippy::map_err_ignore,
    clippy::map_flatten,
    clippy::map_unwrap_or,
    clippy::match_same_arms,
    clippy::match_wildcard_for_single_variants,
    clippy::mem_forget,
    clippy::mut_mut,
    clippy::mutex_integer,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::needless_pass_by_value,
    clippy::option_option,
    clippy::path_buf_push_overwrite,
    clippy::ptr_as_ptr,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::semicolon_if_nothing_returned,
    clippy::string_add_assign,
    clippy::string_add,
    clippy::string_lit_as_bytes,
    clippy::mut_from_ref,
    clippy::missing_transmute_annotations,
    clippy::todo,
    clippy::trait_duplication_in_bounds,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::unused_self,
    clippy::use_self,
    clippy::useless_transmute,
    clippy::verbose_file_reads,
    clippy::wildcard_enum_match_arm,
    clippy::zero_sized_map_values,
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms,
    missing_docs,
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links
)]
#![allow(clippy::single_component_path_imports)]
#![allow(clippy::needless_continue)]

//! Wrappers and raw bindings for Unity's libil2cpp

#[cfg(not(any(
    feature = "il2cpp_v31",
    feature = "il2cpp_v29",
    feature = "il2cpp_v24",
    feature = "unity2018"
)))]
compile_error!("No Unity version selected");

#[cfg(feature = "trace")]
#[macro_use]
extern crate tracing;
#[cfg(feature = "trace")]
pub use tracing::{debug, instrument};

#[cfg(not(feature = "trace"))]
macro_rules! debug {
    ($($tt:tt)*) => {};
}
#[cfg(not(feature = "trace"))]
pub use quest_hook_proc_macros::identity as instrument;

mod array;
mod byref;
mod class;
mod exception;
mod field_info;
mod gc;
mod method_info;
mod object;
mod property_info;
mod valuetype;
#[cfg(feature = "xref")]
mod xref;

mod gc_alloc;

#[cfg_attr(
    any(feature = "unity2018", feature = "il2cpp_v24"),
    path = "parameter_info.rs"
)]
#[cfg_attr(
    any(feature = "il2cpp_v29", feature = "il2cpp_v31"),
    path = "parameter_info_stub.rs"
)]
mod parameter_info;
pub mod raw;
mod string;
pub mod thread;
mod ty;
mod typecheck;

#[doc(inline)]
pub use quest_hook_proc_macros::{unsafe_impl_reference_type, unsafe_impl_value_type};

pub use array::Il2CppArray;
pub use byref::{ByRef, ByRefMut};
pub use class::{FindMethodError, Il2CppClass};
pub use exception::Il2CppException;
pub use field_info::FieldInfo;
pub use method_info::{Il2CppReflectionMethod, MethodInfo, Result, Void};
pub use object::{Il2CppObject, ObjectExt, RefType};
pub use parameter_info::ParameterInfo;
pub use property_info::PropertyInfo;
pub use raw::{unbox, value_box_alloc, WrapRaw, IL2CPP_BINARY};
pub use string::Il2CppString;
pub use ty::{Builtin, Il2CppReflectionType, Il2CppType};
pub use typecheck::callee::{Parameter, Parameters, Return, ThisParameter};
pub use typecheck::caller::{Argument, Arguments, Returned, ThisArgument};
pub use typecheck::generic::Generics;
pub use typecheck::ty::Type;
pub use valuetype::{BoxedValue, ValueType, ValueTypePadding};

#[cfg(feature = "xref")]
pub use xref::xref_init;

pub use crate::gc_alloc::*;
pub use gc::{Gc, NonNullGc};
