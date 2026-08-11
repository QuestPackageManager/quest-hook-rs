use std::fmt::{self, Debug, Formatter};
use std::ops::{Deref, DerefMut, FromResidual, Not, Residual, Try};
use std::ptr::NonNull;

use crate::{RefType, SafePtr, Type};

/// Wrapper type which implies the type is GC managed lifetime
/// This is a Weak pointer to the GC managed object. If you want to hold a
/// strong reference, use [`crate::GcBox<T>`] instead.
///
/// This is nullable, and can be used to represent a null reference to a GC
/// managed object.
#[repr(C)]
pub struct Gc<T>(*mut T);

impl<T> Gc<T> {
    /// Creates a new `Gc` instance with the given pointer.
    pub fn new(ptr: *mut T) -> Self {
        Self(ptr)
    }

    /// Creates a new `Gc` instance with a null pointer.
    pub fn null() -> Self {
        Self::default()
    }

    /// Checks if the pointer is null.
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }

    /// Returns a constant pointer to the value.
    pub fn get_pointer(&self) -> *const T {
        self.0
    }
    /// Returns a mutable pointer to the value.
    pub fn get_pointer_mut(&mut self) -> *mut T {
        self.0
    }

    /// Returns an `Option` containing a reference to the value if the
    /// pointer is not null.
    pub fn as_ref(&self) -> Option<&T> {
        unsafe { self.0.as_ref() }
    }
    /// Returns an `Option` containing a mutable reference to the value if
    /// the pointer is not null.
    pub fn as_mut(&mut self) -> Option<&mut T> {
        unsafe { self.0.as_mut() }
    }

    /// Ensures that the `Gc<T>` is not null, returning a `NonNullGc<T>` if it
    /// is not null, or a `NullGcError` if it is null.
    pub fn ensure_nonnull(&self) -> Result<NonNullGc<T>, NullGcError> {
        NonNull::new(self.0).map(NonNullGc).ok_or(NullGcError)
    }

    /// Casts `T` to `U` using compiler-checked type conversion, which will fail
    /// to compile if `T` is not convertible to `U`. This is a compile-time
    /// checked cast, and will not perform any runtime checks.
    ///
    /// See [`cast`](Gc::cast) for a runtime-checked cast that can fail at
    /// runtime.
    pub fn type_cast<U>(self) -> Gc<U>
    where
        T: AsMut<U>, // ensures T is convertible to U
    {
        if self.is_null() {
            return Gc::null();
        }

        let u = AsMut::as_mut(unsafe { &mut *self.0 });
        Gc(u as *mut U)
    }
}

impl<T> Gc<T>
where
    T: RefType,
{
    /// Casts `T` to `U`, checked against the object's actual runtime class -
    /// can fail ([`Err`]) if it isn't really (assignable to) a `U`. Compare
    /// [`type_cast`](Gc::type_cast), which trusts a compile-time
    /// relationship instead of checking and never fails.
    ///
    /// If the [`Gc<T>`] is null, this will return a null [`Gc<U>`] regardless
    /// of the type relationship.
    ///
    /// C++ Implementation
    /// <https://github.com/QuestPackageManager/beatsaber-hook/blob/2604126ec26dd807da0be0ad974056d1f5fe9575/shared/utils/il2cpp-utils-classes.hpp#L185-L212>
    ///
    /// # Safety
    /// This function is safe to call, but the caller must ensure that the
    /// [`Gc<T>`] is valid and points to a valid object of type `T`.  If the
    /// [`Gc<T>`] points to an invalid object, this function may
    /// cause undefined behavior.
    pub fn cast<U>(mut self) -> Result<Gc<U>, String>
    where
        U: RefType,
        T: RefType,
    {
        let Some(value) = self.as_mut() else {
            return Ok(Gc::null());
        };

        let value_klass = value.as_object().class();

        if value_klass != U::class() && !value_klass.is_assignable_from(U::class()) {
            return Err(format!(
                "Downcast failed: {} is not assignable from {}",
                U::class().name(),
                value_klass.name()
            ));
        }

        // we verified the type is correct, so we can safely cast the pointer
        let cast = (value as *mut T).cast::<U>();
        Ok(Gc(cast))
    }

    /// Converts the current `Gc` instance to a `SafePtr` instance of the same
    /// type.
    pub fn into_safe_ptr(self) -> SafePtr<T> {
        SafePtr::new(self)
    }
}

unsafe impl<T> Type for Gc<T>
where
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    type Held<'a> = Self;

    type HeldRaw = *mut T;

    const NAMESPACE: &'static str = T::NAMESPACE;

    const CLASS_NAME: &'static str = T::CLASS_NAME;

    fn matches_reference_argument(ty: &crate::Il2CppType) -> bool {
        T::matches_reference_argument(ty)
    }

    fn matches_value_argument(ty: &crate::Il2CppType) -> bool {
        T::matches_value_argument(ty)
    }

    fn matches_reference_parameter(ty: &crate::Il2CppType) -> bool {
        T::matches_reference_parameter(ty)
    }

    fn matches_value_parameter(ty: &crate::Il2CppType) -> bool {
        T::matches_value_parameter(ty)
    }
}

impl<T> Try for Gc<T> {
    type Output = Self;

    type Residual = NullGcError;

    fn from_output(output: Self::Output) -> Self {
        output
    }

    fn branch(self) -> std::ops::ControlFlow<Self::Residual, Self::Output> {
        if self.is_null() {
            return std::ops::ControlFlow::Break(NullGcError);
        }
        std::ops::ControlFlow::Continue(self)
    }
}

impl<T> FromResidual<NullGcError> for Gc<T> {
    fn from_residual(_residual: NullGcError) -> Self {
        Gc::null()
    }
}

// Should I do this or force to implement these on a wrapper?
unsafe impl<T> Send for Gc<T> {}
unsafe impl<T> Sync for Gc<T> {}

impl<T> From<Gc<T>> for Option<&T> {
    fn from(value: Gc<T>) -> Self {
        value.is_null().not().then(|| unsafe { &*value.0 })
    }
}
impl<T> From<Gc<T>> for Option<&mut T> {
    fn from(value: Gc<T>) -> Self {
        value.is_null().not().then(|| unsafe { &mut *value.0 })
    }
}

impl<T> PartialEq for Gc<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<T> Eq for Gc<T> {}

impl<T> Clone for Gc<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Gc<T> {}

impl<T> Default for Gc<T> {
    fn default() -> Self {
        Self(std::ptr::null_mut())
    }
}

impl<T> Deref for Gc<T>
where
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        if self.is_null() {
            panic!(
                "Attempted to dereference a null type {}::{}",
                T::NAMESPACE,
                T::CLASS_NAME
            );
        }
        unsafe { &*self.0 }
    }
}
impl<T> DerefMut for Gc<T>
where
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        if self.is_null() {
            panic!(
                "Attempted to dereference a null type {}::{}",
                T::NAMESPACE,
                T::CLASS_NAME
            );
        }
        unsafe { &mut *self.0 }
    }
}

impl<T> AsRef<T> for Gc<T>
where
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn as_ref(&self) -> &T {
        self
    }
}
impl<T> AsMut<T> for Gc<T>
where
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn as_mut(&mut self) -> &mut T {
        self
    }
}

impl<T> From<*mut T> for Gc<T> {
    fn from(ptr: *mut T) -> Self {
        Self(ptr)
    }
}
impl<T> From<*const T> for Gc<T> {
    fn from(ptr: *const T) -> Self {
        Self(ptr as *mut T)
    }
}
impl<T> From<&mut T> for Gc<T> {
    fn from(ptr: &mut T) -> Self {
        Self(ptr)
    }
}
impl<T> From<Option<&mut T>> for Gc<T> {
    fn from(ptr: Option<&mut T>) -> Self {
        match ptr {
            Some(ptr) => Self(ptr),
            None => Self::null(),
        }
    }
}

impl<T> Debug for Gc<T>
where
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if self.is_null() {
            write!(f, "Gc<{}>::null()", T::CLASS_NAME)
        } else {
            write!(f, "Gc<{}>({:p})", T::CLASS_NAME, self.0)
        }
    }
}

/// A [`Gc<T>`] that is statically known to never be null.
///
/// Backed by a [`NonNull<T>`] rather than a bare pointer, so - unlike
/// `Gc<T>` - `Option<NonNullGc<T>>` is free: the compiler represents `None`
/// as the pointer's null bit-pattern instead of needing extra space for a
/// discriminant.
#[repr(transparent)]
pub struct NonNullGc<T>(NonNull<T>);

impl<T> NonNullGc<T> {
    /// Creates a `NonNullGc<T>` from anything convertible to a [`Gc<T>`]
    /// (e.g. a `Gc<T>`, `*mut T`, or `&mut T`), returning `None` if it turns
    /// out to be null.
    pub fn new<Ptr>(gc: Ptr) -> Option<Self>
    where
        Ptr: Into<Gc<T>>,
    {
        let gc = gc.into();

        let nonnull = NonNull::new(gc.0)?;
        Some(Self(nonnull))
    }

    /// Copies out a [`Gc<T>`] equivalent to this `NonNullGc<T>`, without
    /// consuming it. `Gc<T>` is `Copy` (just a pointer), so this is as cheap
    /// as a field access - it exists because `NonNullGc<T>` can't implement
    /// `AsRef<Gc<T>>`/`Deref<Target = Gc<T>>` (it doesn't store a `Gc<T>`
    /// anywhere - only a [`NonNull<T>`], which is what makes
    /// `Option<NonNullGc<T>>` free - so there's no `&Gc<T>` to hand out
    /// without an unsafe layout-reinterpreting cast).
    pub fn as_gc(&self) -> Gc<T> {
        Gc(self.0.as_ptr())
    }
}
unsafe impl<T> Send for NonNullGc<T> where T: for<'a> Type<Held<'a> = Option<&'a mut T>> {}
unsafe impl<T> Sync for NonNullGc<T> where T: for<'a> Type<Held<'a> = Option<&'a mut T>> {}

unsafe impl<T> Type for NonNullGc<T>
where
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    type Held<'a> = Self;

    type HeldRaw = *mut T;

    const NAMESPACE: &'static str = T::NAMESPACE;

    const CLASS_NAME: &'static str = T::CLASS_NAME;

    fn matches_reference_argument(ty: &crate::Il2CppType) -> bool {
        T::matches_reference_argument(ty)
    }

    fn matches_value_argument(ty: &crate::Il2CppType) -> bool {
        T::matches_value_argument(ty)
    }

    fn matches_reference_parameter(ty: &crate::Il2CppType) -> bool {
        T::matches_reference_parameter(ty)
    }

    fn matches_value_parameter(ty: &crate::Il2CppType) -> bool {
        T::matches_value_parameter(ty)
    }
}

impl<T> Deref for NonNullGc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.as_ref() }
    }
}

impl<T> DerefMut for NonNullGc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.as_mut() }
    }
}

impl<T> From<NonNullGc<T>> for Gc<T> {
    fn from(non_null_gc: NonNullGc<T>) -> Self {
        Gc(non_null_gc.0.as_ptr())
    }
}

impl<T> From<&T> for NonNullGc<T> {
    fn from(value: &T) -> Self {
        Self(NonNull::from(value))
    }
}

impl<T> From<&mut T> for NonNullGc<T> {
    fn from(value: &mut T) -> Self {
        Self(NonNull::from(value))
    }
}

impl<T> From<NonNull<T>> for NonNullGc<T> {
    fn from(ptr: NonNull<T>) -> Self {
        Self(ptr)
    }
}

impl<T> From<NonNullGc<T>> for NonNull<T> {
    fn from(non_null_gc: NonNullGc<T>) -> Self {
        non_null_gc.0
    }
}

/// The `Gc<T>` being converted was null.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NullGcError;

impl<T> Residual<Gc<T>> for NullGcError {
    type TryType = Gc<T>;
}

impl fmt::Display for NullGcError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Gc<T> was null")
    }
}

impl std::error::Error for NullGcError {}

impl<T> TryFrom<Gc<T>> for NonNullGc<T> {
    type Error = NullGcError;

    fn try_from(gc: Gc<T>) -> Result<Self, Self::Error> {
        NonNull::new(gc.0).map(Self).ok_or(NullGcError)
    }
}

impl<T> TryFrom<*mut T> for NonNullGc<T> {
    type Error = NullGcError;

    fn try_from(ptr: *mut T) -> Result<Self, Self::Error> {
        NonNull::new(ptr).map(Self).ok_or(NullGcError)
    }
}

#[cfg(feature = "serde")]
mod serde {

    use serde::de::{Deserialize, Deserializer};
    use serde::ser::{Serialize, Serializer};

    use crate::Type;

    use super::Gc;

    impl<'de, T> Deserialize<'de> for Gc<T>
    where
        T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
        for<'a> &'a mut T: Deserialize<'de>,
    {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            let result = <Option<&mut T> as Deserialize>::deserialize(deserializer)?;
            Ok(result.into())
        }
    }

    impl<T> Serialize for Gc<T>
    where
        T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
        for<'a> Option<&'a T>: Serialize,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            <Option<&T> as Serialize>::serialize(&self.as_ref(), serializer)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ptr::NonNull;

    use super::*;
    use crate::{Il2CppClass, Il2CppObject, Il2CppType, WrapRaw};

    /// A stand-in for `UnityEngine.Transform`
    #[repr(C)]
    struct Transform {
        header: crate::raw::Il2CppObject,
        value: i32,
    }

    impl Transform {
        fn new(value: i32) -> Self {
            Self {
                header: crate::raw::Il2CppObject {
                    __bindgen_anon_1: crate::raw::Il2CppObject__bindgen_ty_1 {
                        klass: std::ptr::null_mut(),
                    },
                    monitor: std::ptr::null_mut(),
                },
                value,
            }
        }
    }

    unsafe impl Type for Transform {
        type Held<'a> = Option<&'a mut Transform>;
        type HeldRaw = *mut Transform;

        const NAMESPACE: &'static str = "UnityEngine";
        const CLASS_NAME: &'static str = "Transform";

        fn matches_reference_argument(_ty: &Il2CppType) -> bool {
            unimplemented!()
        }
        fn matches_value_argument(_ty: &Il2CppType) -> bool {
            unimplemented!()
        }
        fn matches_reference_parameter(_ty: &Il2CppType) -> bool {
            unimplemented!()
        }
        fn matches_value_parameter(_ty: &Il2CppType) -> bool {
            unimplemented!()
        }
    }

    impl RefType for Transform {
        fn as_object(&self) -> &Il2CppObject {
            unsafe { Il2CppObject::wrap_ptr(&self.header).unwrap() }
        }
        fn as_object_mut(&mut self) -> &mut Il2CppObject {
            unsafe { Il2CppObject::wrap_ptr_mut(&mut self.header).unwrap() }
        }
    }

    /// A stand-in for `UnityEngine.RectTransform`
    #[repr(C)]
    struct RectTransform {
        transform: Transform,
        #[allow(dead_code)]
        anchor_min: [f32; 2],
    }

    impl AsMut<Transform> for RectTransform {
        fn as_mut(&mut self) -> &mut Transform {
            &mut self.transform
        }
    }

    /// A single, shared, locally leaked fake `Il2CppClass` standing in for
    /// `RectTransform`'s real class - `RectTransform::class()` is
    /// overridden to return it instead of calling `Il2CppClass::find` (which
    /// needs a live il2cpp runtime). `down_cast_succeeds_when_class_matches`
    /// also stamps it directly into a bare `Transform`'s header, simulating
    /// "this `Transform` reference's underlying object is actually a
    /// `RectTransform` at runtime" - enough to exercise `cast`'s
    /// equal-class fast path (`value_klass == U::class()` short-circuits
    /// the `&&` before `is_assignable_from`, the only part of `cast` that
    /// would need a live runtime, to resolve the real
    /// `class_is_assignable_from` FFI function). A genuine (non-identical)
    /// class mismatch isn't testable the same way.
    fn fake_rect_transform_class() -> &'static Il2CppClass {
        static CLASS: std::sync::OnceLock<&'static Il2CppClass> = std::sync::OnceLock::new();
        *CLASS.get_or_init(|| {
            // SAFETY: `Il2CppClass` is a plain-old-data FFI struct (pointers
            // and integers only) - never read through here except for
            // pointer-identity comparisons, which a zeroed instance
            // satisfies just as well as a real one.
            let raw_class: crate::raw::Il2CppClass = unsafe { std::mem::zeroed() };
            let leaked = Box::leak(Box::new(raw_class));
            unsafe { Il2CppClass::wrap_ptr(leaked) }.unwrap()
        })
    }

    unsafe impl Type for RectTransform {
        type Held<'a> = Option<&'a mut RectTransform>;
        type HeldRaw = *mut RectTransform;

        const NAMESPACE: &'static str = "UnityEngine";
        const CLASS_NAME: &'static str = "RectTransform";

        fn class() -> &'static Il2CppClass {
            fake_rect_transform_class()
        }

        fn matches_reference_argument(_ty: &Il2CppType) -> bool {
            unimplemented!()
        }
        fn matches_value_argument(_ty: &Il2CppType) -> bool {
            unimplemented!()
        }
        fn matches_reference_parameter(_ty: &Il2CppType) -> bool {
            unimplemented!()
        }
        fn matches_value_parameter(_ty: &Il2CppType) -> bool {
            unimplemented!()
        }
    }

    impl RefType for RectTransform {
        fn as_object(&self) -> &Il2CppObject {
            unimplemented!()
        }
        fn as_object_mut(&mut self) -> &mut Il2CppObject {
            unimplemented!()
        }
    }

    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn gc_is_send_and_sync() {
        assert_send_sync::<Gc<Transform>>();
    }

    #[test]
    fn new_is_not_null() {
        let mut dummy = Transform::new(42);
        let gc = Gc::new(&mut dummy as *mut Transform);
        assert!(!gc.is_null());
    }

    #[test]
    fn null_and_default_agree() {
        let null_gc: Gc<Transform> = Gc::null();
        assert!(null_gc.is_null());
        assert_eq!(null_gc, Gc::default());
    }

    #[test]
    fn as_ref_reflects_nullness() {
        let mut dummy = Transform::new(7);
        let mut gc = Gc::new(&mut dummy as *mut Transform);
        assert_eq!(gc.as_ref().map(|d| d.value), Some(7));
        assert_eq!(gc.as_mut().map(|d| d.value), Some(7));

        let mut null_gc: Gc<Transform> = Gc::null();
        assert!(null_gc.as_ref().is_none());
        assert!(null_gc.as_mut().is_none());
    }

    #[test]
    fn get_pointer_matches_source() {
        let mut dummy = Transform::new(0);
        let ptr = &mut dummy as *mut Transform;
        let mut gc = Gc::new(ptr);
        assert_eq!(gc.get_pointer(), ptr as *const Transform);
        assert_eq!(gc.get_pointer_mut(), ptr);
    }

    #[test]
    fn deref_reads_and_writes_through_pointer() {
        let mut dummy = Transform::new(99);
        let mut gc = Gc::new(&mut dummy as *mut Transform);
        assert_eq!(gc.value, 99);
        gc.value = 100;
        assert_eq!(dummy.value, 100);
    }

    #[test]
    #[should_panic(expected = "Attempted to dereference a null type UnityEngine::Transform")]
    fn deref_panics_when_null() {
        let gc: Gc<Transform> = Gc::null();
        let _ = &*gc;
    }

    #[test]
    #[should_panic(expected = "Attempted to dereference a null type UnityEngine::Transform")]
    fn deref_mut_panics_when_null() {
        let mut gc: Gc<Transform> = Gc::null();
        let _ = &mut *gc;
    }

    #[test]
    fn clone_copy_and_equality() {
        let mut dummy = Transform::new(1);
        let gc = Gc::new(&mut dummy as *mut Transform);
        let gc_copied = gc;
        let gc_cloned = gc.clone();
        assert_eq!(gc, gc_copied);
        assert_eq!(gc, gc_cloned);

        let other: Gc<Transform> = Gc::null();
        assert_ne!(gc, other);
    }

    #[test]
    fn from_pointer_conversions() {
        let mut dummy = Transform::new(5);
        let ptr: *mut Transform = &mut dummy;

        let gc: Gc<Transform> = ptr.into();
        assert!(!gc.is_null());

        let const_ptr: *const Transform = ptr;
        let gc_from_const: Gc<Transform> = const_ptr.into();
        assert_eq!(gc, gc_from_const);

        let gc_from_mut_ref: Gc<Transform> = (&mut dummy).into();
        assert_eq!(gc, gc_from_mut_ref);
    }

    #[test]
    fn from_option_ref_conversions() {
        let mut dummy = Transform::new(5);

        let gc_some: Gc<Transform> = Some(&mut dummy).into();
        assert!(!gc_some.is_null());

        let gc_none: Gc<Transform> = None::<&mut Transform>.into();
        assert!(gc_none.is_null());
    }

    #[test]
    fn option_ref_conversions_reflect_nullness() {
        let mut dummy = Transform::new(5);
        let gc = Gc::new(&mut dummy as *mut Transform);
        let as_ref: Option<&Transform> = gc.into();
        assert_eq!(as_ref.map(|d| d.value), Some(5));

        let null_gc: Gc<Transform> = Gc::null();
        let as_ref: Option<&Transform> = null_gc.into();
        assert!(as_ref.is_none());

        let as_mut: Option<&mut Transform> = gc.into();
        assert!(as_mut.is_some());
    }

    #[test]
    fn as_ref_as_mut() {
        let mut dummy = Transform::new(3);
        let mut gc = Gc::new(&mut dummy as *mut Transform);
        assert_eq!(AsRef::<Transform>::as_ref(&gc).value, 3);
        AsMut::<Transform>::as_mut(&mut gc).value = 4;
        assert_eq!(dummy.value, 4);
    }

    #[test]
    fn debug_format_distinguishes_null() {
        let null_gc: Gc<Transform> = Gc::null();
        assert_eq!(format!("{:?}", null_gc), "Gc<Transform>::null()");

        let mut dummy = Transform::new(1);
        let gc = Gc::new(&mut dummy as *mut Transform);
        let formatted = format!("{:?}", gc);
        assert_ne!(formatted, "Gc<Transform>::null()");
        assert!(formatted.starts_with("Gc<Transform>("));
    }

    #[test]
    fn try_operator_short_circuits_on_null() {
        fn use_dummy(gc: Gc<Transform>) -> Gc<Transform> {
            let non_null = gc?; // early-returns `Gc::null()` via `FromResidual` if `gc` is null
            non_null
        }

        let mut dummy = Transform::new(5);
        let gc = Gc::new(&mut dummy as *mut Transform);
        assert_eq!(use_dummy(gc), gc);

        let null_gc: Gc<Transform> = Gc::null();
        assert!(use_dummy(null_gc).is_null());
    }

    #[test]
    fn up_cast_on_null_stays_null() {
        let gc: Gc<RectTransform> = Gc::null();
        let up: Gc<Transform> = gc.type_cast();
        assert!(up.is_null());
    }

    #[test]
    fn type_cast_reinterprets_pointer_to_target_type() {
        let mut rect_transform = RectTransform {
            transform: Transform::new(77),
            anchor_min: [0.0, 0.0],
        };
        let ptr = &mut rect_transform as *mut RectTransform;
        let gc = Gc::new(ptr);

        let base: Gc<Transform> = gc.type_cast();
        assert!(!base.is_null());
        // Same address - `RectTransform` embeds `Transform` as its first
        // field, and `type_cast` just reinterprets in place, it doesn't
        // move or allocate.
        assert_eq!(base.get_pointer(), ptr as *const Transform);
        // Sound to read, unlike a layout coincidence would be: this really
        // is a `Transform` living at the front of a real `RectTransform`.
        assert_eq!(base.as_ref().unwrap().value, 77);
    }

    #[test]
    fn down_cast_on_null_stays_null() {
        let gc: Gc<Transform> = Gc::null();
        let down: Gc<RectTransform> = gc.cast().unwrap();
        assert!(down.is_null());
    }

    #[test]
    fn down_cast_succeeds_when_class_matches() {
        // Simulates holding a `Gc<Transform>` whose underlying object's
        // actual runtime type is `RectTransform` (e.g. it's a UI element) -
        // a live il2cpp runtime would have stamped this class pointer into
        // the header at allocation time.
        let mut transform = Transform::new(55);
        transform.header.__bindgen_anon_1.klass =
            fake_rect_transform_class() as *const Il2CppClass as *mut crate::raw::Il2CppClass;
        let ptr = &mut transform as *mut Transform;
        let gc = Gc::new(ptr);

        let cast: Gc<RectTransform> = gc.cast().unwrap();
        assert!(!cast.is_null());
        // Only checking address identity, not dereferencing as a
        // `RectTransform` - the backing allocation here is only ever a bare
        // `Transform`'s worth of memory, so reading `RectTransform`'s extra
        // `anchor_min` field through it would be out of bounds.
        assert_eq!(cast.get_pointer(), ptr as *const RectTransform);
    }

    #[test]
    fn non_null_pointer_to_gc_wrapper_round_trips() {
        let mut dummy = Transform::new(8);
        let mut gc = Gc::new(&mut dummy as *mut Transform);

        // `NonNull<Gc<Transform>>` here is a non-null pointer to the *wrapper*
        // (guaranteed by taking it from a live `&mut`), not a claim about
        // whether the `Gc<Transform>` it points at is the null C# reference.
        let ptr: NonNull<Gc<Transform>> = NonNull::from(&mut gc);
        // SAFETY: `ptr` was just derived from a valid, live `&mut Gc<Transform>`
        // that outlives this call.
        let gc_via_ptr = unsafe { ptr.as_ref() };
        assert_eq!(*gc_via_ptr, gc);
        assert_eq!(gc_via_ptr.as_ref().map(|d| d.value), Some(8));
    }

    #[test]
    fn non_null_wrapper_pointer_does_not_imply_non_null_pointee() {
        // A `NonNull<Gc<T>>` only guarantees the *wrapper*'s address is
        // non-null - a null `Gc<T>` living at a perfectly valid stack
        // address is a legitimate `NonNull<Gc<T>>` target, and dereferencing
        // the pointer still yields a `Gc<T>` that reports `is_null()`.
        let mut null_gc: Gc<Transform> = Gc::null();
        let ptr = NonNull::from(&mut null_gc);
        // SAFETY: `ptr` was just derived from a valid, live `&mut Gc<Transform>`
        // that outlives this call.
        let gc_via_ptr = unsafe { ptr.as_ref() };
        assert!(gc_via_ptr.is_null());
    }

    #[test]
    fn non_null_gc_new_rejects_null_sources() {
        let mut dummy = Transform::new(11);
        let gc = Gc::new(&mut dummy as *mut Transform);
        assert!(NonNullGc::new(gc).is_some());

        let null_gc: Gc<Transform> = Gc::null();
        assert!(NonNullGc::new(null_gc).is_none());

        let null_ptr: *mut Transform = std::ptr::null_mut();
        assert!(NonNullGc::new(null_ptr).is_none());
    }

    #[test]
    fn non_null_gc_derefs_like_gc() {
        let mut dummy = Transform::new(20);
        let gc = Gc::new(&mut dummy as *mut Transform);
        let mut non_null = NonNullGc::new(gc).unwrap();

        // Reads through it the same way `Gc<T>` does.
        assert_eq!(non_null.value, 20);
        // Writes through it the same way `Gc<T>` does.
        non_null.value = 21;
        assert_eq!(dummy.value, 21);
    }

    #[test]
    fn non_null_gc_widens_back_to_the_same_gc() {
        let mut dummy = Transform::new(30);
        let gc = Gc::new(&mut dummy as *mut Transform);
        let non_null = NonNullGc::new(gc).unwrap();

        let widened: Gc<Transform> = non_null.into();
        assert_eq!(widened, gc);
        assert!(!widened.is_null());
    }

    #[test]
    fn non_null_gc_from_references() {
        let mut dummy = Transform::new(40);

        let from_shared: NonNullGc<Transform> = (&dummy).into();
        assert_eq!(from_shared.value, 40);

        let from_mut: NonNullGc<Transform> = (&mut dummy).into();
        assert_eq!(from_mut.value, 40);
    }

    #[test]
    fn non_null_gc_type_matches_gc_type() {
        assert_eq!(
            <NonNullGc<Transform> as Type>::NAMESPACE,
            <Gc<Transform> as Type>::NAMESPACE
        );
        assert_eq!(
            <NonNullGc<Transform> as Type>::CLASS_NAME,
            <Gc<Transform> as Type>::CLASS_NAME
        );
        assert_eq!(
            <NonNullGc<Transform> as Type>::NAMESPACE,
            Transform::NAMESPACE
        );
        assert_eq!(
            <NonNullGc<Transform> as Type>::CLASS_NAME,
            Transform::CLASS_NAME
        );
    }

    #[test]
    fn non_null_gc_is_send_and_sync() {
        assert_send_sync::<NonNullGc<Transform>>();
    }

    #[test]
    fn non_null_gc_is_same_size_as_gc() {
        // `NonNullGc<T>` is `#[repr(transparent)]` over `NonNull<T>` - it
        // should cost nothing over the plain nullable wrapper.
        assert_eq!(
            std::mem::size_of::<NonNullGc<Transform>>(),
            std::mem::size_of::<Gc<Transform>>()
        );
        assert_eq!(
            std::mem::align_of::<NonNullGc<Transform>>(),
            std::mem::align_of::<Gc<Transform>>()
        );
    }

    #[test]
    fn option_non_null_gc_gets_the_real_niche_optimization() {
        // Unlike `Gc<T>` (backed by a plain `*mut T`, which carries no
        // niche), `NonNullGc<T>` is backed by `std::ptr::NonNull<T>` - a
        // type the compiler genuinely niche-optimizes, no unstable
        // attribute required. `Option<NonNullGc<T>>` should therefore stay
        // the same size as `NonNullGc<T>` itself, while `Option<Gc<T>>`
        // does not.
        assert_eq!(
            std::mem::size_of::<Option<NonNullGc<Transform>>>(),
            std::mem::size_of::<NonNullGc<Transform>>()
        );
        assert!(
            std::mem::size_of::<Option<Gc<Transform>>>() > std::mem::size_of::<Gc<Transform>>(),
            "Gc<T> has no niche, so Option<Gc<T>> is expected to be larger"
        );
    }
}
