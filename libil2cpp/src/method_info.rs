use std::borrow::Cow;
use std::ffi::{c_void, CStr};
use std::mem::transmute;
use std::ops::{Deref, DerefMut};
use std::ptr::null_mut;
use std::{fmt, slice};

use crate::raw::{METHOD_ATTRIBUTE_ABSTRACT, METHOD_ATTRIBUTE_STATIC, METHOD_ATTRIBUTE_VIRTUAL};
use crate::{
    raw, Arguments, Gc, Generics, Il2CppClass, Il2CppException, Il2CppObject, Il2CppType,
    ParameterInfo, Returned, ThisArgument, WrapRaw,
};

#[cfg(feature = "il2cpp_v31")]
type ParameterInfoSlice<'a> = &'a [ParameterInfo];

#[cfg(feature = "il2cpp_v29")]
type ParameterInfoSlice<'a> = &'a [ParameterInfo];
#[cfg(feature = "il2cpp_v24")]
type ParameterInfoSlice<'a> = &'a [ParameterInfo];
#[cfg(feature = "unity2018")]
type ParameterInfoSlice<'a> = &'a [&'static ParameterInfo];

/// Information about a C# method
#[repr(transparent)]
pub struct MethodInfo(raw::MethodInfo);

unsafe impl Send for MethodInfo {}
unsafe impl Sync for MethodInfo {}

/// Alias for `()` to represent `void` return type
pub type Void = ();
/// Result type for method invocations
pub type Result<T> = std::result::Result<T, Gc<Il2CppException>>; // Gc<T> adds Sync and Send bounds, allowing for async support

impl MethodInfo {
    /// Invoke this method, type checking against its signature with the
    /// provided instance, arguments and return type
    pub fn invoke<T, A, R, const N: usize>(&self, this: T, args: A) -> Result<R>
    where
        T: ThisArgument,
        A: Arguments<N>,
        R: Returned,
    {
        assert!(T::matches(self));
        assert!(A::matches(self));
        assert!(R::matches(self.return_ty()));

        unsafe { self.invoke_unchecked(this, args) }
    }

    /// Invoke this method with the given instance and arguments and converting
    /// the result to the specified type, without type checking
    ///
    /// # Safety
    /// To be safe, the provided types have to match the method signature
    pub unsafe fn invoke_unchecked<T, A, R, const N: usize>(
        &self,
        mut this: T,
        mut args: A,
    ) -> Result<R>
    where
        T: ThisArgument,
        A: Arguments<N>,
        R: Returned,
    {
        match self.invoke_raw(this.invokable(), args.invokable().as_mut()) {
            Ok(r) => Ok(R::from_object(transmute::<
                Option<&mut raw::Il2CppObject>,
                Option<&mut Il2CppObject>,
            >(r))),
            Err(e) => Err(Il2CppException::wrap_mut(e).into()),
        }
    }

    /// Invokes this method with the given raw instance and arguments, without
    /// performing any checks
    ///
    /// # Safety
    /// To be safe, the provided instance and arguments have to match the method
    /// signature
    pub unsafe fn invoke_raw<'ok, 'err>(
        &self,
        this: *mut c_void,
        args: &mut [*mut c_void],
    ) -> std::result::Result<Option<&'ok mut raw::Il2CppObject>, &'err mut raw::Il2CppException>
    {
        let mut exception = None;
        let r = raw::runtime_invoke(self.raw(), this, args.as_mut_ptr(), &mut exception);
        match exception {
            None => Ok(r),
            Some(e) => Err(e),
        }
    }

    /// [`Il2CppReflectionMethod`] which represents the method
    pub fn reflection_object(&self) -> &Il2CppReflectionMethod {
        unsafe { Il2CppReflectionMethod::wrap_mut(raw::method_get_object(self.raw(), None)) }
    }

    /// Name of the method
    pub fn name(&self) -> Cow<'_, str> {
        let name = self.raw().name;
        assert!(!name.is_null());
        unsafe { CStr::from_ptr(name) }.to_string_lossy()
    }

    /// Class the method is from
    pub fn class(&self) -> &Il2CppClass {
        unsafe { Il2CppClass::wrap_ptr(self.raw().klass) }.unwrap()
    }

    /// Return type of the method
    pub fn return_ty(&self) -> &Il2CppType {
        unsafe { Il2CppType::wrap_ptr(self.raw().return_type).unwrap() }
    }

    /// Parameters the method takes
    pub fn parameters(&self) -> ParameterInfoSlice<'_> {
        let parameters = self.raw().parameters;
        if !parameters.is_null() {
            unsafe { slice::from_raw_parts(parameters.cast(), self.raw().parameters_count as _) }
        } else {
            &[]
        }
    }

    /// Whether the method is static
    pub fn is_static(&self) -> bool {
        self.raw().flags as u32 & METHOD_ATTRIBUTE_STATIC != 0
    }

    /// Whether the method is abstract
    pub fn is_abstract(&self) -> bool {
        self.raw().flags as u32 & METHOD_ATTRIBUTE_ABSTRACT != 0
    }

    /// Whether the method is virtual
    pub fn is_virtual(&self) -> bool {
        self.raw().flags as u32 & METHOD_ATTRIBUTE_VIRTUAL != 0
    }

    /// Whether the method is generic
    pub fn is_generic(&self) -> bool {
        unsafe { raw::method_is_generic(self.raw()) }
    }

    /// The method's vtable slot
    pub fn slot(&self) -> u16 {
        self.raw().slot
    }

    /// Instantiates a generic method template with the provided generic
    /// arguments, returning the resulting concrete method - mirrors
    /// [`Il2CppClass::make_generic`], via
    /// [`Il2CppReflectionMethod::make_generic`]
    /// (`MethodInfo.MakeGenericMethod`).
    /// 
    /// https://github.com/QuestPackageManager/beatsaber-hook/blob/7632eb7bf2634dabbf3cade1df140e5d93f48845/src/types.cpp#L328-L363
    pub fn make_generic<G>(&self) -> Result<Option<&'static Self>>
    where
        G: Generics,
    {
        match self.reflection_object().make_generic::<G>() {
            Ok(Some(refl)) => Ok(Some(unsafe {
                Self::wrap(raw::method_get_from_reflection(refl.raw()))
            })),
            Ok(None) => Ok(None),
            Err(e) => Err(e),
        }
    }
}

unsafe impl WrapRaw for MethodInfo {
    type Raw = raw::MethodInfo;
}

impl fmt::Debug for MethodInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MethodInfo")
            .field("class", self.class())
            .field("name", &self.name())
            .field("parameters", &self.parameters())
            .field("static", &self.is_static())
            .field("abstract", &self.is_abstract())
            .field("virtual", &self.is_virtual())
            .finish()
    }
}

impl fmt::Display for MethodInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let params = self.parameters();

        if self.is_static() {
            f.write_str("static ")?;
        }
        if self.is_abstract() {
            f.write_str("abstract ")?;
        }
        if self.is_virtual() {
            f.write_str("virtual ")?;
        }

        if params.is_empty() {
            write!(f, "{} {}()", self.return_ty(), self.name())
        } else {
            let n = params.len() - 1;

            write!(f, "{} {}(", self.return_ty(), self.name())?;
            for p in &params[..n] {
                write!(f, "{}, ", p)?;
            }
            write!(f, "{})", params[n])
        }
    }
}

/// Object used for reflection of methods
#[repr(transparent)]
pub struct Il2CppReflectionMethod(#[allow(dead_code)] raw::Il2CppReflectionMethod);

impl Il2CppReflectionMethod {
    /// [`MethodInfo`] which this object represents
    pub fn method_info(&self) -> &MethodInfo {
        unsafe { MethodInfo::wrap(raw::method_get_from_reflection(self.raw())) }
    }

    /// Instantiates a generic method template with the provided generic
    /// arguments - mirrors
    /// [`Il2CppReflectionType::make_generic`](crate::Il2CppReflectionType::make_generic),
    /// calling `MethodInfo.MakeGenericMethod` via reflection.
    /// 
    /// https://github.com/QuestPackageManager/beatsaber-hook/blob/7632eb7bf2634dabbf3cade1df140e5d93f48845/src/types.cpp#L113-L133
    pub fn make_generic<G>(&self) -> Result<Option<&Self>>
    where
        G: Generics,
    {
        let generics = G::type_array();
        let make_generic = self
            .class()
            .find_method_unchecked("MakeGenericMethod", 2)
            .unwrap();
        let ret = unsafe {
            make_generic.invoke_raw(
                null_mut(),
                [
                    self as *const Self as *mut c_void,
                    (generics as *mut raw::Il2CppArray).cast(),
                ]
                .as_mut(),
            )
        };
        let obj = match ret {
            Ok(Some(obj)) => obj,
            Ok(None) => return Ok(None),
            Err(e) => return Err(unsafe { Gc::from(Il2CppException::wrap_mut(e)) }),
        };
        let method = unsafe { &mut *(obj as *mut raw::Il2CppObject).cast() };
        Ok(Some(method))
    }
}

impl Deref for Il2CppReflectionMethod {
    type Target = Il2CppObject;

    fn deref(&self) -> &Self::Target {
        unsafe { Il2CppObject::wrap(&self.raw().object) }
    }
}

impl DerefMut for Il2CppReflectionMethod {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { Il2CppObject::wrap_mut(&mut self.raw_mut().object) }
    }
}

impl fmt::Debug for Il2CppReflectionMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Il2CppReflectionMethod")
            .field("method_info", self.method_info())
            .finish()
    }
}

unsafe impl WrapRaw for Il2CppReflectionMethod {
    type Raw = raw::Il2CppReflectionMethod;
}
