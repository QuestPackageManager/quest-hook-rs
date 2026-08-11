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
