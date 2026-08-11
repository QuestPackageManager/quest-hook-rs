use std::fmt::{self, Debug, Formatter};
use std::ops::{Deref, DerefMut};

use crate::{Arguments, Gc, Il2CppObject, RefType, Returned, Type};

/// Extension trait for value types providing additional functionality
pub trait ValueType: for<'a> Type<Held<'a> = Self> + Sized {
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

    /// Converts the value type into a boxed value type, which is a reference
    /// type.
    fn as_boxed(&mut self) -> Gc<BoxedValue<Self>> {
        let boxed = unsafe { crate::value_box_alloc::<Self>(self) };
        boxed
    }
}

impl<T> ValueType for T where T: for<'a> Type<Held<'a> = T> {}

/// A boxed C# value type sitting on the GC heap - the result of boxing a
/// [`ValueType`] (see [`crate::raw::value_box_alloc`]).
///
/// Unlike `T` itself, which is never null/GC-tracked (C# value types are
/// held by value, not by reference - see [`ValueType`]'s `Held<'a> = Self`
/// bound), a `BoxedValue<T>` is reference-shaped, matching how C# actually
/// treats a boxed value: `Gc<BoxedValue<T>>` is the equivalent of a C#
/// `object` holding a boxed `T`. This is why boxing can't just produce a
/// `Gc<T>` directly - `T: ValueType` and `T` satisfying `Gc`'s reference-type
/// bound are mutually exclusive.
///
/// The layout mirrors the real in-memory representation: an
/// [`Il2CppObject`] header immediately followed by `T`'s data.
#[repr(C)]
pub struct BoxedValue<T: ValueType> {
    header: Il2CppObject,
    value: T,
}

impl<T> From<Gc<BoxedValue<T>>> for Option<T>
where
    T: ValueType + Clone,
{
    fn from(boxed: Gc<BoxedValue<T>>) -> Self {
        boxed.as_ref().map(|b| b.value.clone())
    }
}

impl<T> Debug for BoxedValue<T>
where
    T: ValueType + Debug,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "BoxedValue<{}>({:?})", T::CLASS_NAME, self.value)
    }
}

impl<T: ValueType> Deref for BoxedValue<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: ValueType> DerefMut for BoxedValue<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

// `BoxedValue<T>` derefs to its wrapped `T` (above), not to `Il2CppObject`,
// so it doesn't pick up `RefType` from the blanket impl the way most
// reference types do - implemented directly instead, straight off the
// `header` field its layout guarantees.
impl<T: ValueType> RefType for BoxedValue<T> {
    fn as_object(&self) -> &Il2CppObject {
        &self.header
    }

    fn as_object_mut(&mut self) -> &mut Il2CppObject {
        &mut self.header
    }
}

unsafe impl<T: ValueType> Type for BoxedValue<T> {
    type Held<'a> = Option<&'a mut Self>;

    type HeldRaw = *mut Self;

    const NAMESPACE: &'static str = T::NAMESPACE;

    const CLASS_NAME: &'static str = T::CLASS_NAME;

    fn matches_reference_argument(ty: &crate::Il2CppType) -> bool {
        ty.class().is_assignable_from(Self::class())
    }

    fn matches_value_argument(_ty: &crate::Il2CppType) -> bool {
        false
    }

    fn matches_reference_parameter(ty: &crate::Il2CppType) -> bool {
        Self::class().is_assignable_from(ty.class())
    }

    fn matches_value_parameter(_ty: &crate::Il2CppType) -> bool {
        false
    }
}

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
