//! Bulk introspection helpers for exploring the loaded il2cpp domain - ports
//! beatsaber-hook's `debug.hpp` (`log_classes`/`log_methods`/`log_fields`/
//! `log_properties`), useful for reverse-engineering during mod development.
//! Per-item [`Debug`](std::fmt::Debug)/[`Display`](std::fmt::Display) impls
//! already exist on [`Il2CppClass`]/[`MethodInfo`]/[`FieldInfo`]/
//! [`PropertyInfo`]; what's missing without this module is a way to walk
//! *all* of them at once.

use crate::{raw, Il2CppClass, WrapRaw};

/// Iterates every class loaded in the current domain (across every loaded
/// assembly's image), optionally restricted to those whose
/// [`Display`](std::fmt::Display) form (`Namespace::Name`, or
/// `Declaring/Nested` for a nested type) starts with `prefix`.
///
/// <https://github.com/QuestPackageManager/beatsaber-hook/blob/7632eb7bf2634dabbf3cade1df140e5d93f48845/src/debug.cpp#L306-L373>
pub fn classes(prefix: Option<&str>) -> impl Iterator<Item = &'static Il2CppClass> + '_ {
    let domain = unsafe { raw::domain_get() };
    let mut assemblies_count = 0;
    let assemblies = unsafe { raw::domain_get_assemblies(domain, &mut assemblies_count) };

    assemblies
        .iter()
        .take(assemblies_count)
        .filter_map(|assembly| unsafe { raw::assembly_get_image(assembly) })
        .flat_map(|image| {
            let count = unsafe { raw::image_get_class_count(image) };
            (0..count).filter_map(move |i| unsafe { raw::image_get_class(image, i) })
        })
        .map(|class| unsafe { Il2CppClass::wrap(class) })
        .filter(move |class| match prefix {
            Some(prefix) => class.to_string().starts_with(prefix),
            None => true,
        })
}

/// Logs every class in the domain (see [`classes`]), one per line.
///
/// <https://github.com/QuestPackageManager/beatsaber-hook/blob/7632eb7bf2634dabbf3cade1df140e5d93f48845/src/debug.cpp#L306-L373>
pub fn log_classes(prefix: Option<&str>) {
    for class in classes(prefix) {
        debug!("{class}");
    }
}

/// Logs every method belonging to `class`.
///
/// <https://github.com/QuestPackageManager/beatsaber-hook/blob/7632eb7bf2634dabbf3cade1df140e5d93f48845/src/debug.cpp#L428-L456>
pub fn log_methods(class: &Il2CppClass) {
    for method in class.methods() {
        debug!("{method}");
    }
}

/// Logs every field belonging to `class`.
///
/// <https://github.com/QuestPackageManager/beatsaber-hook/blob/7632eb7bf2634dabbf3cade1df140e5d93f48845/src/debug.cpp#L473-L494>
pub fn log_fields(class: &Il2CppClass) {
    for field in class.fields() {
        debug!("{field:?}");
    }
}

/// Logs every property belonging to `class`.
///
/// <https://github.com/QuestPackageManager/beatsaber-hook/blob/7632eb7bf2634dabbf3cade1df140e5d93f48845/src/debug.cpp#L519-L540>
pub fn log_properties(class: &Il2CppClass) {
    for property in class.properties() {
        debug!("{property:?}");
    }
}
