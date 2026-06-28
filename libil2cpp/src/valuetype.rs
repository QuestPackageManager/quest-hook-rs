use crate::{Arguments, Returned, Type};

/// Extension trait for value types providing additional functionality
pub trait ValueTypeExt: for<'a> Type<Held<'a> = Self> + Sized {
    /// Invokes the method with the given name on `self` using the given
    /// arguments, with type checking
    ///
    /// # Panics
    ///
    /// This method will panic if a matching method can't be found.
    fn invoke<A, R, const N: usize>(&mut self, name: &str, args: A) -> crate::Result<R>
    where
        A: Arguments<N>,
        R: Returned,
    {
        let method = Self::class().find_method::<A, R, N>(name).unwrap();
        unsafe { method.invoke_unchecked(self, args) }
    }

    /// Invokes the `void` method with the given name on `self` using the
    /// given arguments, with type checking
    ///
    /// # Panics
    ///
    /// This method will panic if a matching method can't be found.
    fn invoke_void<A, const N: usize>(&mut self, name: &str, args: A) -> crate::Result<()>
    where
        A: Arguments<N>,
    {
        let method = Self::class().find_method::<A, (), N>(name).unwrap();
        unsafe { method.invoke_unchecked(self, args) }
    }
}

impl<T> ValueTypeExt for T where T: for<'a> Type<Held<'a> = T> {}

/// Padding type for value types
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValueTypePadding<const N: usize>(pub [u8; N]);

impl<const N: usize> Default for ValueTypePadding<N> {
    fn default() -> Self {
        Self([0; N])
    }
}

// unsafe_impl_value_type!(in crate for ValueTypePadding<N> =>
// System.ValueType);

unsafe impl<const N: usize> crate::Type for ValueTypePadding<N> {
    type Held<'a> = Self;
    type HeldRaw = Self;
    const NAMESPACE: &'static str = "System";
    const CLASS_NAME: &'static str = "ValueType";
    fn matches_value_argument(ty: &crate::Il2CppType) -> bool {
        !ty.is_ref()
            && ty
                .class()
                .is_assignable_from(<Self as crate::Type>::class())
    }
    fn matches_reference_argument(ty: &crate::Il2CppType) -> bool {
        ty.is_ref()
            && ty
                .class()
                .is_assignable_from(<Self as crate::Type>::class())
    }
    fn matches_value_parameter(ty: &crate::Il2CppType) -> bool {
        !ty.is_ref() && <Self as crate::Type>::class().is_assignable_from(ty.class())
    }
    fn matches_reference_parameter(ty: &crate::Il2CppType) -> bool {
        ty.is_ref() && <Self as crate::Type>::class().is_assignable_from(ty.class())
    }
}
unsafe impl<const N: usize> crate::Argument for ValueTypePadding<N> {
    type Type = Self;
    fn matches(ty: &crate::Il2CppType) -> bool {
        <Self as crate::Type>::matches_value_argument(ty)
    }
    fn invokable(&mut self) -> *mut ::std::ffi::c_void {
        (self as *mut Self).cast::<::std::ffi::c_void>()
    }
}
unsafe impl<const N: usize> crate::Parameter for ValueTypePadding<N> {
    type Actual = Self;
    fn matches(ty: &crate::Il2CppType) -> bool {
        <Self as crate::Type>::matches_value_parameter(ty)
    }
    fn from_actual(actual: Self::Actual) -> Self {
        actual
    }
    fn into_actual(self) -> Self::Actual {
        self
    }
}
unsafe impl<const N: usize> crate::Returned for ValueTypePadding<N> {
    type Type = Self;
    fn matches(ty: &crate::Il2CppType) -> bool {
        <Self as crate::Type>::matches_returned(ty)
    }
    fn from_object(object: Option<&mut crate::Il2CppObject>) -> Self {
        unsafe { crate::raw::unbox(crate::WrapRaw::raw(object.unwrap())) }
    }
}
unsafe impl<const N: usize> crate::Return for ValueTypePadding<N> {
    type Actual = Self;
    fn matches(ty: &crate::Il2CppType) -> bool {
        <Self as crate::Type>::matches_return(ty)
    }
    fn into_actual(self) -> Self::Actual {
        self
    }
    fn from_actual(actual: Self::Actual) -> Self {
        actual
    }
}
