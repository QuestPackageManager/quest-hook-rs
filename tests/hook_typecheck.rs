//! Compile-time checks for the `#[hook(Class::method)]` targeting form:
//! `tests/ui/hook_typecheck/*.rs` are standalone fixtures compiled (and, for
//! the passing case, run) by `trybuild` to confirm the fn-pointer coercion
//! `Metadata::method_check` emits actually catches a mismatched hook
//! signature at compile time, and accepts a matching one.

#[test]
fn hook_typecheck() {
    let t = trybuild::TestCases::new();
    t.pass("tests/ui/hook_typecheck/matching_signature.rs");
    t.compile_fail("tests/ui/hook_typecheck/extra_parameter.rs");
    t.compile_fail("tests/ui/hook_typecheck/wrong_parameter_type.rs");
    t.compile_fail("tests/ui/hook_typecheck/wrong_return_type.rs");
    t.compile_fail("tests/ui/hook_typecheck/wrong_this_type.rs");
}
