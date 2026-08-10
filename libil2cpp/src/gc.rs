use std::fmt::{self, Debug, Formatter};
use std::ops::{Deref, DerefMut, Not};
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

    pub fn as_ref(&self) -> Option<&T> {
        unsafe { self.0.as_ref() }
    }
    pub fn as_mut(&mut self) -> Option<&mut T> {
        unsafe { self.0.as_mut() }
    }
}

impl<T> Gc<T>
where
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    /// Converts the current `Gc` instance to a `Gc` instance of another type.
    ///
    /// # Safety
    /// Relies on the `T` implementation of `AsMut<U>` to be correct.
    pub fn up_cast<U>(mut self) -> Gc<U>
    where
        U: for<'a> Type<Held<'a> = Option<&'a mut U>>,
        T: AsMut<U>, // ensures T is convertible to U
    {
        match self.as_mut() {
            Some(value) => Gc::from(value.as_mut() as &mut U),
            None => Gc::null(),
        }
    }

    /// Converts the current `Gc` instance to a `Gc` instance of another type.
    ///
    /// # Safety
    /// Relies on the `T` implementation of `AsMut<U>` to be correct.
    /// See [`Gc::<T>::up_cast`] for a similar function.
    /// C++ Implementation
    /// <https://github.com/QuestPackageManager/beatsaber-hook/blob/2604126ec26dd807da0be0ad974056d1f5fe9575/shared/utils/il2cpp-utils-classes.hpp#L185-L212>
    pub fn down_cast<U>(mut self) -> Result<Gc<U>, String>
    where
        U: for<'a> Type<Held<'a> = Option<&'a mut U>>,
        T: RefType,
    {
        match self.as_mut() {
            Some(value) => {
                let value_klass = value.as_object().class();

                if value_klass != U::class() && !value_klass.is_assignable_from(U::class()) {
                    return Err(format!(
                        "Downcast failed: {} is not assignable from {}",
                        U::class().name(),
                        value_klass.name()
                    ));
                }

                let cast = (value as *mut T).cast::<U>();
                Ok(Gc(cast))
            }
            None => Ok(Gc::null()),
        }
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

#[repr(transparent)]
pub struct NonNullGc<T>(NonNull<T>);

impl<T> NonNullGc<T> {
    pub fn new<Ptr>(gc: Ptr) -> Option<Self>
    where
        Ptr: Into<Gc<T>>,
    {
        let gc = gc.into();

        let nonnull = NonNull::new(gc.0)?;
        Some(Self(nonnull))
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
    use crate::{Il2CppObject, Il2CppType};

    /// A fake C# type used to exercise `Gc<T>`'s pointer bookkeeping without
    /// needing a live il2cpp runtime. Its `Type`/`RefType`/`AsMut` bodies are
    /// `unimplemented!()` since nothing under test calls into the runtime
    /// (`class()`, `matches_*`, `as_object`) - only the paths that stay
    /// entirely on the Rust side (null checks, deref, casts on a null `Gc`,
    /// equality, conversions) are covered.
    #[repr(C)]
    struct Dummy {
        value: i32,
    }

    unsafe impl Type for Dummy {
        type Held<'a> = Option<&'a mut Dummy>;
        type HeldRaw = *mut Dummy;

        const NAMESPACE: &'static str = "Test";
        const CLASS_NAME: &'static str = "Dummy";

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

    impl AsMut<Dummy> for Dummy {
        fn as_mut(&mut self) -> &mut Dummy {
            self
        }
    }

    impl RefType for Dummy {
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
        assert_send_sync::<Gc<Dummy>>();
    }

    #[test]
    fn new_is_not_null() {
        let mut dummy = Dummy { value: 42 };
        let gc = Gc::new(&mut dummy as *mut Dummy);
        assert!(!gc.is_null());
    }

    #[test]
    fn null_and_default_agree() {
        let null_gc: Gc<Dummy> = Gc::null();
        assert!(null_gc.is_null());
        assert_eq!(null_gc, Gc::default());
    }

    #[test]
    fn as_ref_reflects_nullness() {
        let mut dummy = Dummy { value: 7 };
        let mut gc = Gc::new(&mut dummy as *mut Dummy);
        assert_eq!(gc.as_ref().map(|d| d.value), Some(7));
        assert_eq!(gc.as_mut().map(|d| d.value), Some(7));

        let mut null_gc: Gc<Dummy> = Gc::null();
        assert!(null_gc.as_ref().is_none());
        assert!(null_gc.as_mut().is_none());
    }

    #[test]
    fn get_pointer_matches_source() {
        let mut dummy = Dummy { value: 0 };
        let ptr = &mut dummy as *mut Dummy;
        let mut gc = Gc::new(ptr);
        assert_eq!(gc.get_pointer(), ptr as *const Dummy);
        assert_eq!(gc.get_pointer_mut(), ptr);
    }

    #[test]
    fn deref_reads_and_writes_through_pointer() {
        let mut dummy = Dummy { value: 99 };
        let mut gc = Gc::new(&mut dummy as *mut Dummy);
        assert_eq!(gc.value, 99);
        gc.value = 100;
        assert_eq!(dummy.value, 100);
    }

    #[test]
    #[should_panic(expected = "Attempted to dereference a null type Test::Dummy")]
    fn deref_panics_when_null() {
        let gc: Gc<Dummy> = Gc::null();
        let _ = &*gc;
    }

    #[test]
    #[should_panic(expected = "Attempted to dereference a null type Test::Dummy")]
    fn deref_mut_panics_when_null() {
        let mut gc: Gc<Dummy> = Gc::null();
        let _ = &mut *gc;
    }

    #[test]
    fn clone_copy_and_equality() {
        let mut dummy = Dummy { value: 1 };
        let gc = Gc::new(&mut dummy as *mut Dummy);
        let gc_copied = gc;
        let gc_cloned = gc.clone();
        assert_eq!(gc, gc_copied);
        assert_eq!(gc, gc_cloned);

        let other: Gc<Dummy> = Gc::null();
        assert_ne!(gc, other);
    }

    #[test]
    fn from_pointer_conversions() {
        let mut dummy = Dummy { value: 5 };
        let ptr: *mut Dummy = &mut dummy;

        let gc: Gc<Dummy> = ptr.into();
        assert!(!gc.is_null());

        let const_ptr: *const Dummy = ptr;
        let gc_from_const: Gc<Dummy> = const_ptr.into();
        assert_eq!(gc, gc_from_const);

        let gc_from_mut_ref: Gc<Dummy> = (&mut dummy).into();
        assert_eq!(gc, gc_from_mut_ref);
    }

    #[test]
    fn from_option_ref_conversions() {
        let mut dummy = Dummy { value: 5 };

        let gc_some: Gc<Dummy> = Some(&mut dummy).into();
        assert!(!gc_some.is_null());

        let gc_none: Gc<Dummy> = None::<&mut Dummy>.into();
        assert!(gc_none.is_null());
    }

    #[test]
    fn option_ref_conversions_reflect_nullness() {
        let mut dummy = Dummy { value: 5 };
        let gc = Gc::new(&mut dummy as *mut Dummy);
        let as_ref: Option<&Dummy> = gc.into();
        assert_eq!(as_ref.map(|d| d.value), Some(5));

        let null_gc: Gc<Dummy> = Gc::null();
        let as_ref: Option<&Dummy> = null_gc.into();
        assert!(as_ref.is_none());

        let as_mut: Option<&mut Dummy> = gc.into();
        assert!(as_mut.is_some());
    }

    #[test]
    fn as_ref_as_mut() {
        let mut dummy = Dummy { value: 3 };
        let mut gc = Gc::new(&mut dummy as *mut Dummy);
        assert_eq!(AsRef::<Dummy>::as_ref(&gc).value, 3);
        AsMut::<Dummy>::as_mut(&mut gc).value = 4;
        assert_eq!(dummy.value, 4);
    }

    #[test]
    fn debug_format_distinguishes_null() {
        let null_gc: Gc<Dummy> = Gc::null();
        assert_eq!(format!("{:?}", null_gc), "Gc<Dummy>::null()");

        let mut dummy = Dummy { value: 1 };
        let gc = Gc::new(&mut dummy as *mut Dummy);
        let formatted = format!("{:?}", gc);
        assert_ne!(formatted, "Gc<Dummy>::null()");
        assert!(formatted.starts_with("Gc<Dummy>("));
    }

    #[test]
    fn up_cast_on_null_stays_null() {
        let gc: Gc<Dummy> = Gc::null();
        let up: Gc<Dummy> = gc.up_cast();
        assert!(up.is_null());
    }

    #[test]
    fn down_cast_on_null_stays_null() {
        let gc: Gc<Dummy> = Gc::null();
        let down: Gc<Dummy> = gc.down_cast().unwrap();
        assert!(down.is_null());
    }

    #[test]
    fn non_null_pointer_to_gc_wrapper_round_trips() {
        let mut dummy = Dummy { value: 8 };
        let mut gc = Gc::new(&mut dummy as *mut Dummy);

        // `NonNull<Gc<Dummy>>` here is a non-null pointer to the *wrapper*
        // (guaranteed by taking it from a live `&mut`), not a claim about
        // whether the `Gc<Dummy>` it points at is the null C# reference.
        let ptr: NonNull<Gc<Dummy>> = NonNull::from(&mut gc);
        // SAFETY: `ptr` was just derived from a valid, live `&mut Gc<Dummy>`
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
        let mut null_gc: Gc<Dummy> = Gc::null();
        let ptr = NonNull::from(&mut null_gc);
        // SAFETY: `ptr` was just derived from a valid, live `&mut Gc<Dummy>`
        // that outlives this call.
        let gc_via_ptr = unsafe { ptr.as_ref() };
        assert!(gc_via_ptr.is_null());
    }

    #[test]
    fn non_null_gc_new_rejects_null_sources() {
        let mut dummy = Dummy { value: 11 };
        let gc = Gc::new(&mut dummy as *mut Dummy);
        assert!(NonNullGc::new(gc).is_some());

        let null_gc: Gc<Dummy> = Gc::null();
        assert!(NonNullGc::new(null_gc).is_none());

        let null_ptr: *mut Dummy = std::ptr::null_mut();
        assert!(NonNullGc::new(null_ptr).is_none());
    }

    #[test]
    fn non_null_gc_derefs_like_gc() {
        let mut dummy = Dummy { value: 20 };
        let gc = Gc::new(&mut dummy as *mut Dummy);
        let mut non_null = NonNullGc::new(gc).unwrap();

        // Reads through it the same way `Gc<T>` does.
        assert_eq!(non_null.value, 20);
        // Writes through it the same way `Gc<T>` does.
        non_null.value = 21;
        assert_eq!(dummy.value, 21);
    }

    #[test]
    fn non_null_gc_widens_back_to_the_same_gc() {
        let mut dummy = Dummy { value: 30 };
        let gc = Gc::new(&mut dummy as *mut Dummy);
        let non_null = NonNullGc::new(gc).unwrap();

        let widened: Gc<Dummy> = non_null.into();
        assert_eq!(widened, gc);
        assert!(!widened.is_null());
    }

    #[test]
    fn non_null_gc_from_references() {
        let mut dummy = Dummy { value: 40 };

        let from_shared: NonNullGc<Dummy> = (&dummy).into();
        assert_eq!(from_shared.value, 40);

        let from_mut: NonNullGc<Dummy> = (&mut dummy).into();
        assert_eq!(from_mut.value, 40);
    }

    #[test]
    fn non_null_gc_type_matches_gc_type() {
        assert_eq!(
            <NonNullGc<Dummy> as Type>::NAMESPACE,
            <Gc<Dummy> as Type>::NAMESPACE
        );
        assert_eq!(
            <NonNullGc<Dummy> as Type>::CLASS_NAME,
            <Gc<Dummy> as Type>::CLASS_NAME
        );
        assert_eq!(<NonNullGc<Dummy> as Type>::NAMESPACE, Dummy::NAMESPACE);
        assert_eq!(<NonNullGc<Dummy> as Type>::CLASS_NAME, Dummy::CLASS_NAME);
    }

    #[test]
    fn non_null_gc_is_send_and_sync() {
        assert_send_sync::<NonNullGc<Dummy>>();
    }

    #[test]
    fn non_null_gc_is_same_size_as_gc() {
        // `NonNullGc<T>` is `#[repr(transparent)]` over `NonNull<T>` - it
        // should cost nothing over the plain nullable wrapper.
        assert_eq!(
            std::mem::size_of::<NonNullGc<Dummy>>(),
            std::mem::size_of::<Gc<Dummy>>()
        );
        assert_eq!(
            std::mem::align_of::<NonNullGc<Dummy>>(),
            std::mem::align_of::<Gc<Dummy>>()
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
            std::mem::size_of::<Option<NonNullGc<Dummy>>>(),
            std::mem::size_of::<NonNullGc<Dummy>>()
        );
        assert!(
            std::mem::size_of::<Option<Gc<Dummy>>>() > std::mem::size_of::<Gc<Dummy>>(),
            "Gc<T> has no niche, so Option<Gc<T>> is expected to be larger"
        );
    }
}
