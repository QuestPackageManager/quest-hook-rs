use std::alloc::{AllocError, Allocator, Layout};
use std::ffi::c_void;
use std::fmt::{self, Debug, Formatter};
use std::ops::{Deref, DerefMut};
use std::ptr::NonNull;
use std::sync::Arc;

use crate::raw::{GcAllocFixedFn, GcFreeFixedFn, GcFunctions};
use crate::{Gc, NonNullGc, RefType};

/// An allocator that uses GC functions to allocate memory.
/// This is useful for allocating memory that will be managed by the GC, such as
/// objects that will be used in the Unity engine.
#[derive(Clone, Copy)]
pub struct GcAllocator {
    gc_alloc_fixed: GcAllocFixedFn,
    gc_free_fixed: GcFreeFixedFn,
}

impl GcAllocator {
    /// Create a new `GcAllocator` instance. This will fail if the GC functions
    /// have not been resolved yet.
    pub fn new() -> Result<Self, String> {
        let gc_functions = GcFunctions::get().ok_or("GC functions not initialized")?;
        Ok(Self {
            gc_alloc_fixed: gc_functions
                .gc_alloc_fixed
                .ok_or("gc_alloc_fixed function not resolved")?,
            gc_free_fixed: gc_functions
                .gc_free_fixed
                .ok_or("gc_free_fixed function not resolved")?,
        })
    }
}

unsafe impl Allocator for GcAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        let ptr = unsafe { (self.gc_alloc_fixed)(layout.size()) };
        if ptr.is_null() {
            return Err(AllocError);
        }
        Ok(NonNull::slice_from_raw_parts(
            NonNull::new(ptr as *mut u8).unwrap(),
            layout.size(),
        ))
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, _layout: Layout) {
        (self.gc_free_fixed)(ptr.as_ptr() as *mut c_void);
    }
}
/// Box with [`GcAllocator`].
pub type GcBox<T> = Box<T, GcAllocator>;
/// Vec with [`GcAllocator`].
pub type GcVec<T> = Vec<T, GcAllocator>;
/// HashMap with [`GcAllocator`].
pub type GcHashMap<K, V> = std::collections::HashMap<K, V, GcAllocator>;
/// Rc with [`GcAllocator`].
pub type GcRc<T> = std::rc::Rc<T, GcAllocator>;
/// Arc with [`GcAllocator`].
pub type GcArc<T> = std::sync::Arc<T, GcAllocator>;

/// A strong reference to a GC-managed il2cpp object, safe to hold onto
/// across GC collections (unlike [`Gc<T>`], a weak, untracked pointer whose
/// pointee the GC may collect at any time) and safe to share across
/// threads.
///
/// This is never null - the inner pointer is a [`NonNullGc<T>`], enforced at
/// construction (see [`SafePtr::new`]).
///
/// Rather than moving or copying the pointee, this roots it by putting a
/// [`NonNullGc<T>`] in a `gc_alloc_fixed`-backed allocation (see
/// [`GcAllocator`]) - the Boehm collector traces GC-fixed allocations when
/// scanning for live references, so the allocation's mere (continued)
/// existence keeps the pointee alive. It's refcounted natively (via `Arc`,
/// not GC memory) and freed once the last `SafePtr<T>` sharing it drops.
///
/// This is essentially an [`Arc<NonNullGc<T>>`] with GC-aware rooting baked
/// in. Because the handle is strongly typed to `T` (not type-erased),
/// [`up_cast`](SafePtr::up_cast)/[`down_cast`](SafePtr::down_cast) can't
/// reuse the same allocation across the type change the way a type-erased
/// handle could - each produces a `SafePtr<U>` backed by its own fresh
/// root, independently keeping the pointee alive rather than sharing a
/// refcount with the `SafePtr<T>` it was cast from.
///
/// Mirrors beatsaber-hook's
/// [`safe_ptr<T>`](https://github.com/QuestPackageManager/beatsaber-hook/blob/7632eb7bf2634dabbf3cade1df140e5d93f48845/shared/safeptr.hpp).
pub struct SafePtr<T>
where
    T: RefType,
{
    handle: GcArc<NonNullGc<T>>,
}

impl<T> SafePtr<T>
where
    T: RefType,
{
    /// Roots `ptr`'s pointee for as long as this `SafePtr<T>` (or any clone
    /// of it) exists.
    ///
    /// # Panics
    /// Panics if `ptr` is null, or if `GcAllocator` isn't initialized yet.
    pub fn new(ptr: Gc<T>) -> Self {
        let ptr = NonNullGc::new(ptr).expect("SafePtr::new: pointer was null");

        let allocator = GcAllocator::new().expect("GcAllocator not initialized");
        let handle: GcArc<NonNullGc<T>> = Arc::new_in(ptr, allocator);

        Self { handle }
    }

    /// Casts `T` to `U` using compiler-checked type conversion, which will fail
    /// to compile if `T` is not convertible to `U`. This is a compile-time
    /// checked cast, and will not perform any runtime checks.
    ///
    /// See [`cast`](SafePtr::<T>::cast) for a runtime-checked cast that can
    /// fail at runtime.
    ///
    /// Cannot be null, since `SafePtr<T>` is never null (see [`SafePtr::new`]).
    ///
    /// See [`Gc::type_cast`] for more details.
    pub fn type_cast<U>(&self) -> SafePtr<U>
    where
        U: RefType,
        T: AsMut<U>, // ensures T is convertible to U
    {
        SafePtr::new(self.handle.as_gc().type_cast::<U>())
    }

    /// Casts `T` to `U`, checked against the object's actual runtime class -
    /// see [`Gc::cast`], which this calls before re-rooting the result
    /// in a new [`SafePtr<U>`].
    ///
    /// This cannot be null, since [`SafePtr<T>`] is never null (see
    /// [`SafePtr::new`]).
    ///
    /// # Safety
    /// This function is safe to call, but the caller must ensure that the
    /// [`Gc<T>`] is valid and points to a valid object of type `T`.
    ///  If the [`Gc<T>`] points to an invalid object, this function
    /// may cause undefined behavior.
    ///
    /// See [`Gc::cast`].
    pub fn cast<U>(&self) -> Result<SafePtr<U>, String>
    where
        U: RefType,
        T: RefType,
    {
        let downcasted_ptr = self.handle.as_gc().cast::<U>()?;
        Ok(SafePtr::new(downcasted_ptr))
    }

    /// Returns a weak reference to the underlying [`Gc<T>`] pointer.
    pub fn as_weak(&self) -> Gc<T> {
        self.handle.as_gc()
    }
}

impl<T> Clone for SafePtr<T>
where
    T: RefType,
{
    fn clone(&self) -> Self {
        Self {
            handle: Arc::clone(&self.handle),
        }
    }
}

impl<T> PartialEq for SafePtr<T>
where
    T: RefType,
{
    fn eq(&self, other: &Self) -> bool {
        self.handle.as_gc() == other.handle.as_gc()
    }
}

impl<T> Eq for SafePtr<T> where T: RefType {}

impl<T> Deref for SafePtr<T>
where
    T: RefType,
{
    type Target = T;

    fn deref(&self) -> &T {
        // `self.handle` (`Arc<NonNullGc<T>>`) derefs to `&NonNullGc<T>`,
        // which itself derefs to `&T` - rooted for as long as `self.handle`
        // is alive (see the struct's doc comment), and never null (see
        // `NonNullGc<T>`).
        &**self.handle
    }
}

impl<T> DerefMut for SafePtr<T>
where
    T: RefType,
{
    fn deref_mut(&mut self) -> &mut T {
        // `Arc` never hands out `&mut` to its payload (shared ownership),
        // so this goes around it via the raw pointer instead - sound for
        // the same reason `Gc<T>::deref_mut` is: `self.handle` keeps the
        // pointee rooted, and it's never null (see `NonNullGc<T>`).
        unsafe { &mut *(self.handle.as_gc().get_pointer() as *mut T) }
    }
}

impl<T> Debug for SafePtr<T>
where
    T: RefType,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "SafePtr<{}>({:?})", T::CLASS_NAME, self.handle.as_gc())
    }
}

impl<T> From<SafePtr<T>> for Gc<T>
where
    T: RefType,
{
    fn from(safe_ptr: SafePtr<T>) -> Self {
        safe_ptr.handle.as_gc()
    }
}

impl<T> From<Gc<T>> for SafePtr<T>
where
    T: RefType,
{
    fn from(gc: Gc<T>) -> Self {
        Self::new(gc)
    }
}

#[cfg(test)]
// See the matching comment in gc.rs's test module - the `unimplemented!()`
// bodies below are deliberate stand-ins, not oversights.
#[allow(clippy::unimplemented)]
mod tests {
    use super::*;
    use crate::{Il2CppObject, Il2CppType, Type};

    /// A stand-in for `UnityEngine.Transform` - matching the one in `gc.rs`'s
    /// tests, just enough of a `Type` impl to satisfy `SafePtr<T>`'s bound.
    /// Its `matches_*`/`AsRef`/`AsMut` bodies are `unimplemented!()` since
    /// nothing under test calls into the runtime.
    #[repr(C)]
    struct Transform {
        #[allow(dead_code)]
        value: i32,
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

    impl AsRef<Il2CppObject> for Transform {
        fn as_ref(&self) -> &Il2CppObject {
            unimplemented!()
        }
    }

    impl AsMut<Il2CppObject> for Transform {
        fn as_mut(&mut self) -> &mut Il2CppObject {
            unimplemented!()
        }
    }

    /// A stand-in for `UnityEngine.RectTransform`, a real subclass of
    /// `UnityEngine.Transform` - genuinely *embeds* a `Transform` as its
    /// first field (real C# single-inheritance layout), matching `gc.rs`'s
    /// tests, rather than two unrelated types that only coincidentally share
    /// a layout. Gives `type_cast`/`cast`'s `U` type parameter something
    /// concrete to monomorphize against in the tests below.
    #[repr(C)]
    struct RectTransform {
        #[allow(dead_code)]
        transform: Transform,
    }

    unsafe impl Type for RectTransform {
        type Held<'a> = Option<&'a mut RectTransform>;
        type HeldRaw = *mut RectTransform;

        const NAMESPACE: &'static str = "UnityEngine";
        const CLASS_NAME: &'static str = "RectTransform";

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

    impl AsRef<Il2CppObject> for RectTransform {
        fn as_ref(&self) -> &Il2CppObject {
            unimplemented!()
        }
    }

    impl AsMut<Il2CppObject> for RectTransform {
        fn as_mut(&mut self) -> &mut Il2CppObject {
            unimplemented!()
        }
    }

    impl AsMut<Transform> for RectTransform {
        fn as_mut(&mut self) -> &mut Transform {
            &mut self.transform
        }
    }

    fn assert_send_sync<T: Send + Sync>() {}

    // Most of `SafePtr<T>`'s actual behavior (construction, `Clone`ing a
    // shared root, `Deref`, `up_cast`/`down_cast`) can't be exercised here:
    // `SafePtr::new` calls `GcAllocator::new()`, which needs
    // `GcFunctions::get()` to have been resolved against a *live* il2cpp
    // binary loaded in this process (see `raw::gc::GcFunctions::init`) -
    // nothing a plain `cargo test` run on the host can provide. Even the
    // fixture-backed integration test in `tests/gc_alloc.rs` only goes as
    // far as constructing a `GcAllocator` - it deliberately stops short of
    // actually allocating, since the real GC heap isn't initialized just
    // from loading the library. So what's covered here is only what's true
    // regardless of runtime state: the type-level `Send`/`Sync` contract,
    // and that failing to allocate fails loudly instead of doing something
    // unsound.

    #[test]
    fn safe_ptr_is_send_and_sync() {
        assert_send_sync::<SafePtr<Transform>>();
    }

    #[test]
    fn gc_allocator_new_fails_cleanly_without_a_live_runtime() {
        // No `GcFunctions::init` has run in this process, so this must
        // report failure rather than e.g. resolving null function pointers.
        assert!(GcAllocator::new().is_err());
    }

    #[test]
    #[should_panic(expected = "GcAllocator not initialized")]
    fn safe_ptr_new_panics_without_a_live_runtime() {
        let mut transform = Transform { value: 0 };
        let gc = Gc::new(&mut transform as *mut Transform);
        // Panics inside `SafePtr::new` (not UB, not a silent no-op) once it
        // reaches the `GcAllocator::new().expect(...)` call.
        let _ = SafePtr::new(gc);
    }

    // `type_cast`/`cast` themselves are even less reachable than the rest of
    // `SafePtr<T>`: both take `&self`, so there's no way to call either
    // without first getting past `SafePtr::new` - which, per the test above,
    // always panics on a plain `cargo test` run. The two tests below can't
    // exercise the casts' *runtime* behavior for that reason, but the calls
    // are still fully monomorphized and typechecked against concrete `U`
    // types even though they never execute - so they still catch a bound or
    // signature regression on `SafePtr::type_cast`/`SafePtr::cast` (the kind
    // this file has drifted into more than once while `Gc<T>`'s own
    // `type_cast`/`cast` were being redesigned).

    #[test]
    #[should_panic(expected = "GcAllocator not initialized")]
    fn safe_ptr_type_cast_is_unreachable_without_a_live_runtime() {
        // Up-casting `RectTransform` to its real base class `Transform`.
        let mut rect_transform = RectTransform {
            transform: Transform { value: 0 },
        };
        let gc = Gc::new(&mut rect_transform as *mut RectTransform);
        let _: SafePtr<Transform> = SafePtr::new(gc).type_cast();
    }

    #[test]
    #[should_panic(expected = "GcAllocator not initialized")]
    fn safe_ptr_cast_is_unreachable_without_a_live_runtime() {
        // Down-casting a generic `Transform` handle to `RectTransform`.
        let mut transform = Transform { value: 0 };
        let gc = Gc::new(&mut transform as *mut Transform);
        let _: Result<SafePtr<RectTransform>, String> = SafePtr::new(gc).cast();
    }
}
