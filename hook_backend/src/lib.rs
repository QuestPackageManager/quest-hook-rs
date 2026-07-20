#![doc(html_root_url = "https://stackdoubleflow.github.io/quest-hook-rs/hook_backend")]
#![warn(
    clippy::all,
    clippy::await_holding_lock,
    clippy::char_lit_as_u8,
    clippy::checked_conversions,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_markdown,
    clippy::empty_enums,
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
    unexpected_cfgs,
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

//! A cross platform function hooking abstraction, working across Windows,
//! Linux, macOS and Android
//!
//! The `Hook` implementation is chosen at compile time via Cargo features:
//! - `inline_hook`: a vendored And64InlineHook/inlineHook.c backend for
//!   AArch64 and ARMv7 Android.
//! - `flamingo`: a [`flamingo_rs`] backend for AArch64 Android.
//! - `retour`: a [`retour`] backend, used on non-Android targets.

use cfg_if::cfg_if;

#[cfg(all(
    target_arch = "aarch64",
    target_os = "android",
    feature = "inline_hook",
    feature = "flamingo"
))]
compile_error!(
    "hook_backend: enable only one of `inline_hook` or `flamingo` for aarch64-android"
);

cfg_if! {
    if #[cfg(all(target_arch = "aarch64", target_os = "android", feature = "flamingo"))] {
        mod aarch64_flamingo;
        pub use crate::aarch64_flamingo::*;
    } else if #[cfg(all(target_arch = "aarch64", target_os = "android", feature = "inline_hook"))] {
        mod aarch64_linux_android;
        pub use crate::aarch64_linux_android::*;
    } else if #[cfg(all(target_arch = "arm", target_os = "android", feature = "inline_hook"))] {
        mod armv7_linux_androideabi;
        pub use crate::armv7_linux_androideabi::*;
    } else if #[cfg(feature = "retour")] {
        mod detour;
        pub use crate::detour::*;
    } else {
        compile_error!(
            "hook_backend: no hooking backend feature enabled for this target; enable `inline_hook`, `flamingo`, or `retour`"
        );
    }
}

#[cfg(test)]
mod tests {
    use std::mem::transmute;

    use super::Hook;

    #[test]
    fn target_and_original() {
        static HOOK: Hook = Hook::new();

        #[inline(never)]
        fn add(n1: usize, n2: usize) -> usize {
            n1 + n2
        }

        #[inline(never)]
        fn mul(n1: usize, n2: usize) -> usize {
            n1 * n2
        }

        assert_eq!(add(2, 3), 5);
        assert_eq!(mul(2, 3), 6);

        assert!(unsafe { HOOK.install(add as _, mul as _) } && HOOK.is_installed());

        assert_eq!(add(2, 3), mul(2, 3));

        let original =
            unsafe { transmute::<*const (), fn(usize, usize) -> usize>(HOOK.original().unwrap()) };
        assert_eq!(original(2, 3), 5);
    }
}
