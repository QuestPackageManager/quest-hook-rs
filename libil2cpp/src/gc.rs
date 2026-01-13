use std::fmt::{self, Debug, Formatter};
use std::mem::transmute;
use std::ops::{Deref, DerefMut, Not};

use crate::{Argument, ObjectType, Returned, ThisArgument, Type};

/// Wrapper type which implies the type is GC managed lifetime
#[repr(C)]
pub struct Gc<T>(*mut T)
where
    *mut T: GcType, // assert that *mut T is a GcType
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>;

/// Trait alias for types that can be used with the `Gc` wrapper.
pub trait GcType = Type + Returned + ThisArgument + Argument;

impl<T> Gc<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
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

    /// Returns an `Option` containing a reference to the value if the pointer
    /// is not null.
    pub fn as_opt(&self) -> Option<&T> {
        self.is_null().not().then(|| unsafe { &*self.0 })
    }
    /// Returns an `Option` containing a mutable reference to the value if the
    /// pointer is not null.
    pub fn as_opt_mut(&mut self) -> Option<&mut T> {
        self.is_null().not().then(|| unsafe { &mut *self.0 })
    }

    /// Returns a constant pointer to the value.
    pub fn get_pointer(&self) -> *const T {
        self.0
    }
    /// Returns a mutable pointer to the value.
    pub fn get_pointer_mut(&mut self) -> *mut T {
        self.0
    }

    /// Converts the current `Gc` instance to a `Gc` instance of another type.
    ///
    /// # Safety
    /// Relies on the `T` implementation of `AsMut<U>` to be correct.
    pub fn cast<U>(mut self) -> Gc<U>
    where
        *mut U: GcType,
        U: for<'a> Type<Held<'a> = Option<&'a mut U>>,
        T: AsMut<U>, // ensures T is convertible to U
    {
        match self.as_opt_mut() {
            Some(value) => Gc::from(value.as_mut() as &mut U),
            None => Gc::null(),
        }
    }
    /// Converts the current `Gc` instance to a `Gc` instance of another type.
    ///
    /// # Safety
    /// Relies on the `T` implementation of `AsMut<U>` to be correct.
    ///
    /// C++ Implementation
    /// <https://github.com/QuestPackageManager/beatsaber-hook/blob/2604126ec26dd807da0be0ad974056d1f5fe9575/shared/utils/il2cpp-utils-classes.hpp#L185-L212>
    pub fn down_cast<U>(mut self) -> Result<Gc<U>, String>
    where
        *mut U: GcType,
        U: for<'a> Type<Held<'a> = Option<&'a mut U>>,
        T: ObjectType,
    {
        match self.as_opt_mut() {
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
}

unsafe impl<T> Type for Gc<T>
where
    *mut T: GcType,
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
unsafe impl<T> Send for Gc<T> where T: for<'a> Type<Held<'a> = Option<&'a mut T>> {}
unsafe impl<T> Sync for Gc<T> where T: for<'a> Type<Held<'a> = Option<&'a mut T>> {}

impl<T> From<Gc<T>> for Option<&T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn from(value: Gc<T>) -> Self {
        value.is_null().not().then(|| unsafe { &*value.0 })
    }
}
impl<T> From<Gc<T>> for Option<&mut T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn from(value: Gc<T>) -> Self {
        value.is_null().not().then(|| unsafe { &mut *value.0 })
    }
}

impl<T> PartialEq for Gc<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<T> Eq for Gc<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
}

impl<T> Clone for Gc<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Gc<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
}

impl<T> Default for Gc<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn default() -> Self {
        Self(std::ptr::null_mut())
    }
}

impl<T> Deref for Gc<T>
where
    *mut T: GcType,
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
    *mut T: GcType,
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
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn as_ref(&self) -> &T {
        self
    }
}
impl<T> AsMut<T> for Gc<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn as_mut(&mut self) -> &mut T {
        self
    }
}

impl<T> From<*mut T> for Gc<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn from(ptr: *mut T) -> Self {
        Self(ptr)
    }
}
impl<T> From<*const T> for Gc<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn from(ptr: *const T) -> Self {
        Self(ptr as *mut T)
    }
}
impl<T> From<&mut T> for Gc<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn from(ptr: &mut T) -> Self {
        Self(ptr)
    }
}
impl<T> From<Option<&mut T>> for Gc<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn from(ptr: Option<&mut T>) -> Self {
        match ptr {
            Some(ptr) => Self(ptr),
            None => Self::null(),
        }
    }
}

impl<T> Debug for Gc<T>
where
    *mut T: GcType,
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

#[cfg(feature = "serde")]
mod serde {

    use serde::de::{Deserialize, Deserializer};
    use serde::ser::{Serialize, Serializer};

    use crate::Type;

    use super::{Gc, GcType};

    impl<'de, T> Deserialize<'de> for Gc<T>
    where
        *mut T: GcType,
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
        *mut T: GcType,
    {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            <Option<&T> as Serialize>::serialize(&self.as_opt(), serializer)
        }
    }
}
