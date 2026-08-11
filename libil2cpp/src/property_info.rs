use std::borrow::Cow;
use std::ffi::CStr;
use std::fmt;

use crate::{raw, Argument, Il2CppClass, MethodInfo, Returned, ThisArgument, WrapRaw};

/// Information about a C# property
#[repr(transparent)]
pub struct PropertyInfo(raw::PropertyInfo);

unsafe impl Send for PropertyInfo {}
unsafe impl Sync for PropertyInfo {}

impl PropertyInfo {
    /// Gets a typechecked value from the property on `instance`
    ///
    /// # Panics
    ///
    /// This method will panic if the property has no getter
    pub fn get<T, R>(&self, instance: T) -> crate::Result<R>
    where
        T: ThisArgument,
        R: Returned,
    {
        let getter = self
            .getter()
            .unwrap_or_else(|| panic!("property {} has no getter", self.name()));
        getter.invoke(instance, ())
    }

    /// Gets a value from the property on `instance`, without type checking
    ///
    /// # Panics
    ///
    /// This method will panic if the property has no getter
    ///
    /// # Safety
    /// To be safe, the provided types have to match the property's getter
    /// signature
    pub unsafe fn get_unchecked<T, R>(&self, instance: T) -> crate::Result<R>
    where
        T: ThisArgument,
        R: Returned,
    {
        let getter = self
            .getter()
            .unwrap_or_else(|| panic!("property {} has no getter", self.name()));
        unsafe { getter.invoke_unchecked(instance, ()) }
    }

    /// Sets a typechecked value on the property on `instance`
    ///
    /// # Panics
    ///
    /// This method will panic if the property has no setter
    pub fn set<T, A>(&self, instance: T, value: A) -> crate::Result<()>
    where
        T: ThisArgument,
        A: Argument,
    {
        let setter = self
            .setter()
            .unwrap_or_else(|| panic!("property {} has no setter", self.name()));
        setter.invoke(instance, value)
    }

    /// Sets a value on the property on `instance`, without type checking
    ///
    /// # Panics
    ///
    /// This method will panic if the property has no setter
    ///
    /// # Safety
    /// To be safe, the provided types have to match the property's setter
    /// signature
    pub unsafe fn set_unchecked<T, A>(&self, instance: T, value: A) -> crate::Result<()>
    where
        T: ThisArgument,
        A: Argument,
    {
        let setter = self
            .setter()
            .unwrap_or_else(|| panic!("property {} has no setter", self.name()));
        unsafe { setter.invoke_unchecked(instance, value) }
    }

    /// The property's getter method, if it has one
    pub fn getter(&self) -> Option<&'static MethodInfo> {
        unsafe { MethodInfo::wrap_ptr(self.raw().get) }
    }

    /// The property's setter method, if it has one
    pub fn setter(&self) -> Option<&'static MethodInfo> {
        unsafe { MethodInfo::wrap_ptr(self.raw().set) }
    }

    /// Name of the property
    pub fn name(&self) -> Cow<'_, str> {
        let name = self.raw().name;
        assert!(!name.is_null());
        unsafe { CStr::from_ptr(name) }.to_string_lossy()
    }

    /// Class the property is from
    pub fn parent(&self) -> &Il2CppClass {
        unsafe { Il2CppClass::wrap_ptr(self.raw().parent) }.unwrap()
    }
}

unsafe impl WrapRaw for PropertyInfo {
    type Raw = raw::PropertyInfo;
}

impl fmt::Debug for PropertyInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PropertyInfo")
            .field("name", &self.name())
            .field("getter", &self.getter())
            .field("setter", &self.setter())
            .finish()
    }
}

// These tests build hand-populated raw structs rather than loading a real
// il2cpp binary (see `libil2cpp/tests/README.md`/`gc.rs`'s test module for
// why) - `get`/`set` only need to prove they panic before ever reaching the
// getter/setter `MethodInfo`, since actually invoking one needs a live
// runtime and is out of scope for a unit test.
#[cfg(test)]
mod tests {
    use std::mem;

    use super::*;

    fn leak<T>(value: T) -> &'static T {
        Box::leak(Box::new(value))
    }

    /// A fake class, only ever used as a pointer target for identity/debug
    /// purposes - never actually read through.
    fn fake_class() -> &'static Il2CppClass {
        let raw: raw::Il2CppClass = unsafe { mem::zeroed() };
        unsafe { Il2CppClass::wrap_ptr(leak(raw)) }.unwrap()
    }

    fn fake_method(name: &'static CStr) -> &'static MethodInfo {
        let mut raw: raw::MethodInfo = unsafe { mem::zeroed() };
        raw.name = name.as_ptr();
        // Give it a valid (if fake) parent class so `MethodInfo::class()` -
        // called from `MethodInfo`'s `Debug` impl - doesn't panic on a null
        // pointer.
        raw.klass = fake_class().raw() as *const raw::Il2CppClass as *mut raw::Il2CppClass;
        unsafe { MethodInfo::wrap_ptr(leak(raw)) }.unwrap()
    }

    fn fake_property(
        name: &'static CStr,
        parent: &'static Il2CppClass,
        get: Option<&'static MethodInfo>,
        set: Option<&'static MethodInfo>,
    ) -> &'static PropertyInfo {
        let mut raw: raw::PropertyInfo = unsafe { mem::zeroed() };
        raw.name = name.as_ptr();
        raw.parent = parent.raw() as *const raw::Il2CppClass as *mut raw::Il2CppClass;
        raw.get = get.map_or(std::ptr::null(), |m| m.raw() as *const raw::MethodInfo);
        raw.set = set.map_or(std::ptr::null(), |m| m.raw() as *const raw::MethodInfo);
        unsafe { PropertyInfo::wrap_ptr(leak(raw)) }.unwrap()
    }

    #[test]
    fn name_and_parent() {
        let class = fake_class();
        let prop = fake_property(c"Health", class, None, None);

        assert_eq!(prop.name(), "Health");
        assert!(std::ptr::eq(prop.parent(), class));
    }

    #[test]
    fn getter_and_setter_reflect_presence() {
        let class = fake_class();
        let accessor = fake_method(c"get_Health");

        let both = fake_property(c"Health", class, Some(accessor), Some(accessor));
        assert!(both.getter().is_some());
        assert!(both.setter().is_some());

        let neither = fake_property(c"ReadOnly", class, None, None);
        assert!(neither.getter().is_none());
        assert!(neither.setter().is_none());
    }

    #[test]
    #[should_panic(expected = "property NoGetter has no getter")]
    fn get_panics_without_a_getter() {
        let class = fake_class();
        let prop = fake_property(c"NoGetter", class, None, None);
        let _: crate::Result<()> = prop.get(());
    }

    // `set`/`set_unchecked`'s no-setter panic isn't separately tested:
    // `Argument` has no impl for `()`, and every real implementor needs a
    // full `RefType` (see `gc.rs`'s test module for what that mock takes),
    // for no extra coverage - `set` panics via the exact same
    // `unwrap_or_else` pattern as `get`, which is exercised above.

    #[test]
    fn debug_includes_name() {
        let class = fake_class();
        let prop = fake_property(c"Health", class, None, None);

        let formatted = format!("{prop:?}");
        assert!(formatted.contains("Health"));
    }
}
