use crate::{raw, Il2CppClass, Il2CppReflectionType, Type, WrapRaw};

/// Trait implemented for Rust types which can represent a list of C# generic
/// arguments
pub trait Generics {
    /// Number of generic arguments
    const COUNT: usize;

    /// Returns an array of `System.RuntimeType`s matching the generic arguments
    fn type_array() -> &'static mut raw::Il2CppArray;

    /// The generic arguments' classes, in order - used to substitute a
    /// generic method's un-instantiated parameter types (`T`, `U`, ...) with
    /// concrete ones for matching, before it's actually been instantiated
    /// with [`MethodInfo::make_generic`](crate::MethodInfo::make_generic).
    ///
    /// Returns `Vec` rather than a `[&'static Il2CppClass; Self::COUNT]`
    /// array - sizing an array by an associated const like `Self::COUNT` in
    /// a trait method signature needs the `generic_const_exprs` feature,
    /// which is unstable enough (frequent ICEs, incomplete) that it's not
    /// worth enabling crate-wide just for this; the allocation here is
    /// negligible next to the FFI calls already happening alongside it.
    /// [`Arguments<const N: usize>`](crate::Arguments) sidesteps the same
    /// problem by parameterizing the *trait* by `N` instead of using an
    /// associated const, which would work here too, but would mean
    /// threading a second const-generic through every `G: Generics` call
    /// site (`find_generic`, `make_generic`,
    /// [`Il2CppClass::find_method`](crate::Il2CppClass::find_method), ...).
    fn classes() -> Vec<&'static Il2CppClass>;
}

impl Generics for () {
    const COUNT: usize = 0;

    fn type_array() -> &'static mut raw::Il2CppArray {
        unsafe { raw::array_new(Il2CppReflectionType::class().raw(), 0) }.unwrap()
    }

    fn classes() -> Vec<&'static Il2CppClass> {
        Vec::new()
    }
}

impl<T: Type> Generics for T {
    const COUNT: usize = 1;

    fn type_array() -> &'static mut raw::Il2CppArray {
        let arr = unsafe { raw::array_new(Il2CppReflectionType::class().raw(), 1) }.unwrap();
        unsafe {
            (((arr as *mut _ as isize) + (raw::kIl2CppSizeOfArray as isize))
                as *mut &Il2CppReflectionType)
                .write_unaligned(T::class().ty().reflection_object());
        }
        arr
    }

    fn classes() -> Vec<&'static Il2CppClass> {
        vec![T::class()]
    }
}
