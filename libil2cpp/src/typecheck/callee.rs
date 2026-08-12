use std::fmt;

use crate::byref::ReffableType;
use crate::{Builtin, ByRef, Gc, Il2CppClass, Il2CppType, MethodInfo, RefType, Type};

/// Trait implemented by types that can be used as C# `this` method parameters
///
/// # Note
/// You should most likely not be implementing this trait yourself, but rather
/// the [`Type`] trait
///
/// # Safety
/// The implementation must be correct
pub unsafe trait ThisParameter {
    /// Type of the actual `this` parameter
    type Actual;

    /// Checks whether the type can be used as a C# instance parameter for the
    /// given [`MethodInfo`]
    fn matches(method: &MethodInfo) -> bool;

    /// Converts from the actual type to the desired one
    fn from_actual(actual: Self::Actual) -> Self;
    /// Converts from the desired type into the actual one
    fn into_actual(self) -> Self::Actual;
}

/// Trait implemented by types that can be used as C# method parameters
///
/// # Note
/// You should most likely not be implementing this trait yourself, but rather
/// the [`Type`] trait
///
/// # Safety
/// The implementation must be correct
pub unsafe trait Parameter {
    /// Type of the actual parameter
    type Actual;

    /// Checks whether the type can be used as a C# parameter with the given
    /// [`Il2CppType`]
    fn matches(ty: &Il2CppType) -> bool;

    /// [`Il2CppClass`] of the parameter's static type - used to rank
    /// candidate overloads by how close a match they are, mirroring
    /// [`Argument::class`](crate::Argument::class)
    fn class() -> &'static Il2CppClass;

    /// Converts from the actual type to the desired one
    fn from_actual(actual: Self::Actual) -> Self;
    /// Converts from the desired type into the actual one
    fn into_actual(self) -> Self::Actual;
}

/// Trait implemented by types that can be used as return types for C#
/// methods
///
/// # Note
/// You should most likely not be implementing this trait yourself, but rather
/// the [`Type`] trait
///
/// # Safety
/// The implementation must be correct
pub unsafe trait Return {
    /// Type of the actual return value
    type Actual;

    /// Checks whether the type can be used as a C# return type of the given
    /// [`Il2CppType`]
    fn matches(ty: &Il2CppType) -> bool;

    /// Converts from the desired type into the actual one
    fn into_actual(self) -> Self::Actual;
    /// Converts from the actual type to the desired one
    fn from_actual(actual: Self::Actual) -> Self;
}

/// Trait implemented by types that can be used as a collection of C# method
/// parameters
///
/// # Note
/// You should most likely not be implementing this trait yourself
///
/// # Safety
/// The implementation must be correct
pub unsafe trait Parameters {
    /// Parameter count
    const COUNT: usize;

    /// Checks whether the type can be used as a C# parameter collection for the
    /// given [`MethodInfo`]
    fn matches_method(method: &MethodInfo) -> bool {
        method.parameters().len() == Self::COUNT
            && Self::matches(
                &method
                    .parameters()
                    .iter()
                    .map(|p| p.ty())
                    .collect::<Vec<_>>(),
            )
    }

    /// Checks whether the type can be used as a C# parameter collection
    /// against an explicit list of [`Il2CppType`]s, rather than pulling them
    /// from a [`MethodInfo`]'s declared parameters directly like
    /// [`matches_method`](Parameters::matches_method) does - used by
    /// [`Il2CppClass::find_method_callee`](crate::Il2CppClass::find_method_callee)
    /// to type-check parameters after any generic-parameter substitution,
    /// mirroring [`Arguments::matches`](crate::Arguments::matches).
    fn matches(types: &[&Il2CppType]) -> bool;

    /// [`Il2CppClass`]es of each parameter's static type, in order - see
    /// [`Parameter::class`]
    fn classes() -> Vec<&'static Il2CppClass>;
}

unsafe impl<T> ThisParameter for Option<&mut T>
where
    T: Type,
{
    type Actual = Self;

    fn matches(method: &MethodInfo) -> bool {
        T::matches_this_parameter(method)
    }

    fn from_actual(actual: Self::Actual) -> Self {
        actual
    }
    fn into_actual(self) -> Self::Actual {
        self
    }
}
unsafe impl<T> ThisParameter for *mut T
where
    T: Type,
{
    type Actual = Self;

    fn matches(method: &MethodInfo) -> bool {
        T::matches_this_parameter(method)
    }

    fn from_actual(actual: Self::Actual) -> Self {
        actual
    }
    fn into_actual(self) -> Self::Actual {
        self
    }
}

unsafe impl<T> ThisParameter for &mut T
where
    T: Type,
{
    type Actual = Option<Self>;

    fn matches(method: &MethodInfo) -> bool {
        T::matches_this_parameter(method)
    }

    fn from_actual(actual: Self::Actual) -> Self {
        actual.unwrap()
    }
    fn into_actual(self) -> Self::Actual {
        Some(self)
    }
}

unsafe impl ThisParameter for () {
    type Actual = !;

    fn matches(method: &MethodInfo) -> bool {
        method.is_static()
    }

    fn from_actual(_: !) {
        unreachable!();
    }
    fn into_actual(self) -> ! {
        unreachable!()
    }
}

// TODO: Remove this once rustfmt stops dropping generics on GATs
#[rustfmt::skip]
unsafe impl<T> Parameter for Option<&mut T>
where
    T: RefType,
{
    type Actual = Self;

    fn matches(ty: &Il2CppType) -> bool {
        T::matches_reference_parameter(ty)
    }

    fn class() -> &'static Il2CppClass {
        T::class()
    }

    fn from_actual(actual: Self::Actual) -> Self {
        actual
    }
    fn into_actual(self) -> Self::Actual {
        self
    }
}
#[rustfmt::skip]
unsafe impl<T> Parameter for *mut T
where
    T: RefType,
{
    type Actual = Self;

    fn matches(ty: &Il2CppType) -> bool {
        T::matches_reference_parameter(ty)
    }

    fn class() -> &'static Il2CppClass {
        T::class()
    }

    fn from_actual(actual: Self::Actual) -> Self {
        actual
    }
    fn into_actual(self) -> Self::Actual {
        self
    }
}

// TODO: Remove this once rustfmt stops dropping generics on GATs
#[rustfmt::skip]
unsafe impl<T> Parameter for &mut T
where
    T: RefType,
{
    type Actual = Option<Self>;

    fn matches(ty: &Il2CppType) -> bool {
        T::matches_reference_parameter(ty)
    }

    fn class() -> &'static Il2CppClass {
        T::class()
    }

    fn from_actual(actual: Self::Actual) -> Self {
        actual.unwrap()
    }
    fn into_actual(self) -> Self::Actual {
        Some(self)
    }
}
// TODO: Remove this once rustfmt stops dropping generics on GATs
#[rustfmt::skip]
unsafe impl<T> ThisParameter for Gc<T>
where
    T: RefType,
{
    type Actual = Self;

    fn matches(ty: &MethodInfo) -> bool {
        T::matches_this_parameter(ty)
    }

    fn from_actual(actual: Self::Actual) -> Self {
        actual
    }
    fn into_actual(self) -> Self::Actual {
        self
    }
}
#[rustfmt::skip]
unsafe impl<T> ThisParameter for ByRef<T>
where
    T: ReffableType,
{
    type Actual = Self;

    fn matches(ty: &MethodInfo) -> bool {
        T::matches_this_parameter(ty)
    }

    fn from_actual(actual: Self::Actual) -> Self {
        actual
    }
    fn into_actual(self) -> Self::Actual {
        self
    }
}

// TODO: Remove this once rustfmt stops dropping generics on GATs
#[rustfmt::skip]
unsafe impl<T> Parameter for Gc<T>
where
    T: RefType,
{
    type Actual = Self;

    fn matches(ty: &Il2CppType) -> bool {
        T::matches_reference_parameter(ty)
    }

    fn class() -> &'static Il2CppClass {
        T::class()
    }

    fn from_actual(actual: Self::Actual) -> Self {
        actual
    }
    fn into_actual(self) -> Self::Actual {
        self
    }
}
#[rustfmt::skip]
unsafe impl<T> Parameter for ByRef<T>
where T: ReffableType,
{
    type Actual = Self;

    fn matches(ty: &Il2CppType) -> bool {
        T::matches_reference_parameter(ty)
    }

    fn class() -> &'static Il2CppClass {
        <T as Type>::class()
    }

    fn from_actual(actual: Self::Actual) -> Self {
        actual
    }
    fn into_actual(self) -> Self::Actual {
        self
    }
}

// TODO: Remove this once rustfmt stops dropping generics on GATs
#[rustfmt::skip]
unsafe impl<T> Return for Gc<T>
where
    T: RefType,
{
    type Actual = Self;

    fn matches(ty: &Il2CppType) -> bool {
        T::matches_return(ty)
    }

    fn into_actual(self) -> Self::Actual {
        self
    }
    fn from_actual(actual: Self::Actual) -> Self {
        actual
    }
}
// TODO: Remove this once rustfmt stops dropping generics on GATs
#[rustfmt::skip]
unsafe impl<T> Return for ByRef<T>
where T: ReffableType
{
    type Actual = Self;

    fn matches(ty: &Il2CppType) -> bool {
        T::matches_return(ty)
    }

    fn into_actual(self) -> Self::Actual {
        self
    }
    fn from_actual(actual: Self::Actual) -> Self {
        actual
    }
}

// TODO: Remove this once rustfmt stops dropping generics on GATs
#[rustfmt::skip]
unsafe impl<T> Return for Option<&mut T>
where
    T: RefType,
{
    type Actual = Self;

    fn matches(ty: &Il2CppType) -> bool {
        T::matches_return(ty)
    }

    fn into_actual(self) -> Self::Actual {
        self
    }
    fn from_actual(actual: Self::Actual) -> Self {
        actual
    }
}
#[rustfmt::skip]
unsafe impl<T> Return for *mut T
where
    T: RefType,
{
    type Actual = Self;

    fn matches(ty: &Il2CppType) -> bool {
        T::matches_return(ty)
    }

    fn into_actual(self) -> Self::Actual {
        self
    }
    fn from_actual(actual: Self::Actual) -> Self {
        actual
    }
}

// TODO: Remove this once rustfmt stops dropping generics on GATs
#[rustfmt::skip]
unsafe impl<T> Return for &mut T
where
    T: RefType,
{
    type Actual = Option<Self>;

    fn matches(ty: &Il2CppType) -> bool {
        T::matches_return(ty)
    }

    fn into_actual(self) -> Self::Actual {
        Some(self)
    }
    fn from_actual(actual: Self::Actual) -> Self {
        actual.unwrap()
    }
}

unsafe impl Return for () {
    type Actual = ();

    fn matches(ty: &Il2CppType) -> bool {
        ty.is_builtin(Builtin::Void)
    }

    fn into_actual(self) {}
    fn from_actual((): ()) {}
}

unsafe impl<T, E> Return for Result<T, E>
where
    T: Return,
    E: fmt::Debug,
{
    type Actual = T::Actual;

    fn matches(ty: &Il2CppType) -> bool {
        T::matches(ty)
    }

    fn into_actual(self) -> Self::Actual {
        self.unwrap().into_actual()
    }
    fn from_actual(actual: Self::Actual) -> Self {
        Ok(T::from_actual(actual))
    }
}

unsafe impl Parameters for () {
    const COUNT: usize = 0;

    fn matches(types: &[&Il2CppType]) -> bool {
        types.is_empty()
    }

    fn classes() -> Vec<&'static Il2CppClass> {
        Vec::new()
    }
}

unsafe impl<P> Parameters for P
where
    P: Parameter,
{
    const COUNT: usize = 1;

    fn matches(types: &[&Il2CppType]) -> bool {
        matches!(types, [ty] if P::matches(ty))
    }

    fn classes() -> Vec<&'static Il2CppClass> {
        vec![P::class()]
    }
}
