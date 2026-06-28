use crate::{
    Il2CppClass, Il2CppException, Il2CppObject, Il2CppReflectionMethod, Il2CppReflectionType,
    Il2CppString, Il2CppType, MethodInfo,
};

/// Trait implemented by Rust types that are also C# types
///
/// # Safety
/// The Rust and C# types must be ABI-compatible and the trait implementation
/// must be correct
pub unsafe trait Type: 'static {
    /// Type of the values held in variables of the type
    type Held<'a>;
    /// Non-generic version of [`Held`].
    type HeldRaw;

    /// Namespace containingthe class the type represents
    const NAMESPACE: &'static str;
    /// Name of the class the type represents
    const CLASS_NAME: &'static str;

    /// [`Il2CppClass`] of the type
    fn class() -> &'static Il2CppClass {
        Il2CppClass::find(Self::NAMESPACE, Self::CLASS_NAME)
            .unwrap_or_else(|| panic!("Class {}.{} not found", Self::NAMESPACE, Self::CLASS_NAME))
    }

    /// Returns the [`Il2CppType`] of this type
    fn type_() -> &'static Il2CppType {
        Self::class().ty()
    }

    /// Whether the type can be used as a `this` argument for the given
    /// [`MethodInfo`]
    fn matches_this_argument(method: &MethodInfo) -> bool {
        method.class().is_assignable_from(Self::class())
    }

    /// Whether the type can be used as a `this` parameter for the given
    /// [`MethodInfo`]
    fn matches_this_parameter(method: &MethodInfo) -> bool {
        Self::class().is_assignable_from(method.class())
    }

    /// Whether a reference to the type can be used as an argument of the given
    /// [`Il2CppType`]
    fn matches_reference_argument(ty: &Il2CppType) -> bool;
    /// Whether a value of the type can be used as an argument of the given
    /// [`Il2CppType`]
    fn matches_value_argument(ty: &Il2CppType) -> bool;

    /// Whether a reference to the type can be used as a parameter of the given
    /// [`Il2CppType`]
    fn matches_reference_parameter(ty: &Il2CppType) -> bool;
    /// Whether a value of the type can be used as a parameter of the given
    /// [`Il2CppType`]
    fn matches_value_parameter(ty: &Il2CppType) -> bool;

    /// Whether the type can be used as the value of the given  [`Il2CppType`]
    /// returned from a C# method
    fn matches_returned(ty: &Il2CppType) -> bool {
        Self::class().is_assignable_from(ty.class())
    }

    /// Whether the type can be used as the return value of the  given
    /// [`Il2CppType`] for a C# method
    fn matches_return(ty: &Il2CppType) -> bool {
        ty.class().is_assignable_from(Self::class())
    }
}

// implement type for pointers
unsafe impl<T: Type> Type for *mut T {
    type Held<'a> = Option<&'a mut T>;
    type HeldRaw = *mut T;

    const NAMESPACE: &'static str = T::NAMESPACE;
    const CLASS_NAME: &'static str = T::CLASS_NAME;

    fn matches_reference_argument(ty: &Il2CppType) -> bool {
        T::matches_reference_argument(ty)
    }

    fn matches_value_argument(ty: &Il2CppType) -> bool {
        T::matches_value_argument(ty)
    }

    fn matches_reference_parameter(ty: &Il2CppType) -> bool {
        T::matches_reference_parameter(ty)
    }

    fn matches_value_parameter(ty: &Il2CppType) -> bool {
        T::matches_value_argument(ty)
    }
}
unsafe impl<T: Type> Type for *const T {
    type Held<'a> = Option<&'a mut T>;
    type HeldRaw = *mut T;

    const NAMESPACE: &'static str = T::NAMESPACE;
    const CLASS_NAME: &'static str = T::CLASS_NAME;

    fn matches_reference_argument(ty: &Il2CppType) -> bool {
        T::matches_reference_argument(ty)
    }

    fn matches_value_argument(ty: &Il2CppType) -> bool {
        T::matches_value_argument(ty)
    }

    fn matches_reference_parameter(ty: &Il2CppType) -> bool {
        T::matches_reference_parameter(ty)
    }

    fn matches_value_parameter(ty: &Il2CppType) -> bool {
        T::matches_value_argument(ty)
    }
}

crate::unsafe_impl_value_type!(in crate for u8 => System.Byte);
crate::unsafe_impl_value_type!(in crate for i8 => System.SByte);
crate::unsafe_impl_value_type!(in crate for u16 => System.UInt16);
crate::unsafe_impl_value_type!(in crate for i16 => System.Int16);
crate::unsafe_impl_value_type!(in crate for u32 => System.UInt32);
crate::unsafe_impl_value_type!(in crate for i32 => System.Int32);
crate::unsafe_impl_value_type!(in crate for u64 => System.UInt64);
crate::unsafe_impl_value_type!(in crate for i64 => System.Int64);
crate::unsafe_impl_value_type!(in crate for usize => System.UIntPtr);
crate::unsafe_impl_value_type!(in crate for isize => System.IntPtr);
crate::unsafe_impl_value_type!(in crate for f32 => System.Single);
crate::unsafe_impl_value_type!(in crate for f64 => System.Double);
crate::unsafe_impl_value_type!(in crate for bool => System.Boolean);
crate::unsafe_impl_value_type!(in crate for char => System.Char);

crate::unsafe_impl_reference_type!(in crate for Il2CppException => System.Exception);
crate::unsafe_impl_reference_type!(in crate for Il2CppObject => System.Object);
crate::unsafe_impl_reference_type!(in crate for Il2CppString => System.String);
crate::unsafe_impl_reference_type!(in crate for Il2CppReflectionType => System.RuntimeType);
crate::unsafe_impl_reference_type!(in crate for Il2CppReflectionMethod => System.Reflection.MonoMethod);
