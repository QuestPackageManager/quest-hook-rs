pub mod callee;
pub mod caller;
pub mod generic;
pub mod ty;

use std::ffi::c_void;

use crate::{Argument, Arguments, Generics, Il2CppClass, Il2CppType, Parameter, Parameters, Type};

quest_hook_proc_macros::impl_arguments_parameters!(1..=32);
quest_hook_proc_macros::impl_generics!(1..=32);
