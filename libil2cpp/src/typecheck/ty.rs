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

    /// Whether the type can be used as the implicit `this` argument when
    /// Rust code calls the given C# instance [`MethodInfo`] - i.e. whether
    /// `Self` is (or derives from) the method's declaring class.
    ///
    /// "Argument" means Rust is the caller here, supplying `this` to native
    /// code; see [`matches_this_parameter`](Type::matches_this_parameter)
    /// for the reverse direction.
    fn matches_this_argument(method: &MethodInfo) -> bool {
        method.class().is_assignable_from(Self::class())
    }

    /// Whether the type can receive the implicit `this` argument when C#
    /// code calls into Rust as the given instance [`MethodInfo`]'s
    /// implementation.
    ///
    /// The callee-side mirror of
    /// [`matches_this_argument`](Type::matches_this_argument) - the
    /// assignability check is reversed because caller and callee swap roles
    /// (here `Self` is the expected/declared type and `method`'s class is
    /// the value being checked against it, rather than the other way
    /// around).
    fn matches_this_parameter(method: &MethodInfo) -> bool {
        Self::class().is_assignable_from(method.class())
    }

    /// Whether a *reference* to the type (a GC pointer, e.g. a
    /// [`Gc<T>`](crate::Gc)) can be passed as an argument of the given
    /// [`Il2CppType`] when Rust calls a C# method.
    ///
    /// "Argument" means Rust is the caller, supplying the value to native
    /// code. True C# reference types (classes, boxed values, strings, ...) are
    /// always passed by pointer and so implement this by checking
    /// assignability alone, ignoring [`Il2CppType::is_ref`] entirely (see
    /// [`unsafe_impl_reference_type!`](crate::unsafe_impl_reference_type)).
    /// 
    /// A C# *value* type parameter declared `ref`/`out`/`in` is passed by
    /// pointer too, though, so [`ValueType`](crate::ValueType)s also
    /// implement this - but only when `ty.is_ref()` is set, since an
    /// ordinary (non-byref) value-type parameter must go by value instead
    /// (see [`matches_value_argument`](Type::matches_value_argument)).
    fn matches_reference_argument(ty: &Il2CppType) -> bool;

    /// Whether a *value* of the type (the raw struct bytes, not a pointer)
    /// can be passed as an argument of the given [`Il2CppType`] when Rust
    /// calls a C# method.
    ///
    /// Only meaningful for [`ValueType`](crate::ValueType)s, which match
    /// when `ty` is *not* declared `ref`/`out`/`in` (see
    /// [`Il2CppType::is_ref`]) and is assignable - a byref value-type
    /// parameter is passed by pointer instead and is matched by
    /// [`matches_reference_argument`](Type::matches_reference_argument).
    /// True reference types unconditionally return `false` here, since their
    /// representation is never raw bytes at the ABI boundary.
    fn matches_value_argument(ty: &Il2CppType) -> bool;

    /// Whether a *reference* to the type can be received as a parameter of
    /// the given [`Il2CppType`] when C# calls into Rust.
    ///
    /// The callee-side mirror of
    /// [`matches_reference_argument`](Type::matches_reference_argument) -
    /// see that method for what "reference" means and why
    /// [`ValueType`](crate::ValueType)s only implement this when
    /// [`ty.is_ref()`](Il2CppType::is_ref) is set. As with
    /// [`matches_this_parameter`](Type::matches_this_parameter), the
    /// assignability check is reversed relative to the argument-side
    /// version because caller and callee swap roles.
    fn matches_reference_parameter(ty: &Il2CppType) -> bool;
    
    /// Whether a *value* of the type can be received as a parameter of the
    /// given [`Il2CppType`] when C# calls into Rust.
    ///
    /// The callee-side mirror of
    /// [`matches_value_argument`](Type::matches_value_argument) - see that
    /// method for why only non-byref [`ValueType`](crate::ValueType)s match
    /// and why true reference types always return `false` here.
    fn matches_value_parameter(ty: &Il2CppType) -> bool;

    /// Whether the type can hold the value of the given [`Il2CppType`] when
    /// it comes back as the return value of a C# method Rust called.
    ///
    /// Rust is the caller here, receiving the value; see
    /// [`matches_return`](Type::matches_return) for the callee-side check
    /// used when a Rust function is invoked as a C# method's implementation.
    fn matches_returned(ty: &Il2CppType) -> bool {
        Self::class().is_assignable_from(ty.class())
    }

    /// Whether the type can be returned as the given [`Il2CppType`] when
    /// this Rust function is invoked as a C# method's implementation, handing
    /// the value back to native code.
    ///
    /// The callee-side mirror of [`matches_returned`](Type::matches_returned)
    /// - the assignability check is reversed for the same reason as
    /// [`matches_this_parameter`](Type::matches_this_parameter).
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
