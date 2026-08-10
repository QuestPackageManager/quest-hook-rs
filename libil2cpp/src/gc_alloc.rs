use std::alloc::{AllocError, Allocator, Layout};
use std::ffi::c_void;
use std::fmt::{self, Debug, Formatter};
use std::ops::{Deref, DerefMut, Not};
use std::ptr::NonNull;
use std::sync::Arc;

use crate::raw::{GcAllocFixedFn, GcFreeFixedFn, GcFunctions};
use crate::{Gc, GcType, ObjectType, Type};

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

/// A tiny GC-fixed allocation holding just a pointer to the actual rooted
/// instance. Because it lives in the Boehm GC's own heap (allocated via the
/// same `gc_alloc_fixed` [`GcAllocator`] wraps), the collector traces
/// through it when scanning for live references - so as long as this
/// allocation itself isn't freed, its `inst` pointer can't be collected.
///
/// Kept alive (and freed once the last [`SafePtr<T>`] sharing it drops) by
/// storing it as a plain [`GcArc<Wrapper>`] - `Arc<T, GcAllocator>` already
/// puts its refcounts and `T` in one `gc_alloc_fixed`-backed allocation and
/// deallocates correctly on its own, so there's no need for a hand-rolled
/// handle type, a custom `Drop` impl, or a separate `Box` layer.
///
/// Mirrors beatsaber-hook's `safe_ptr<T>::wrapper`.
#[repr(C)]
struct Wrapper {
    inst: *mut c_void,
}

// SAFETY: `Wrapper` is just a pointer value, never dereferenced on its own
// (only ever read back out and cast, in `SafePtr::new`/`Deref`) - nothing
// about accessing it from another thread is unsound.
unsafe impl Send for Wrapper {}
unsafe impl Sync for Wrapper {}

/// A strong reference to a GC-managed il2cpp object, safe to hold onto
/// across GC collections (unlike [`Gc<T>`], a weak, untracked pointer whose
/// pointee the GC may collect at any time) and safe to share across
/// threads.
/// 
/// This is nullable, see [`Gc<T>`].
/// 
///
/// Rather than moving or copying the pointee, this roots it by allocating a
/// tiny [`Wrapper`] block holding its pointer via the Boehm GC's own fixed
/// allocator (see [`GcAllocator`]) - the collector traces GC-fixed
/// allocations when scanning for live references, so the wrapper's mere
/// (continued) existence keeps the pointee alive. The wrapper is refcounted
/// natively (via `Arc`, not GC memory) and freed once the last `SafePtr<T>`
/// sharing it drops.
///
/// This is basically a Arc<Gc<T>, GcAllocator> with useful ergonomics and reduced allocations. We have an easier time
/// with the type erased `Wrapper` managing the GC root for when casting between types.
/// 
/// Mirrors beatsaber-hook's
/// [`safe_ptr<T>`](https://github.com/QuestPackageManager/beatsaber-hook/blob/7632eb7bf2634dabbf3cade1df140e5d93f48845/shared/safeptr.hpp).
pub struct SafePtr<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    ptr: Gc<T>,
    handle: GcArc<Wrapper>,
}

// SAFETY: the pointee is rooted (see the `Wrapper` doc above) for as long as
// `handle` (shared, refcounted) is alive, regardless of which thread drops
// the last reference or dereferences `ptr`.
unsafe impl<T> Send for SafePtr<T> where T: for<'a> Type<Held<'a> = Option<&'a mut T>> {}
unsafe impl<T> Sync for SafePtr<T> where T: for<'a> Type<Held<'a> = Option<&'a mut T>> {}

impl<T> SafePtr<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    /// Roots `ptr`'s pointee for as long as this `SafePtr<T>` (or any clone
    /// of it) exists.
    ///
    /// # Panics
    /// Panics if `ptr` is null, or if `GcAllocator` isn't initialized yet.
    pub fn new(ptr: Gc<T>) -> Self {
        assert!(!ptr.is_null(), "SafePtr::new: pointer was null");
        let ptr = ptr.get_pointer() as *mut T;

        let allocator = GcAllocator::new().expect("GcAllocator not initialized");
        let handle: GcArc<Wrapper> = Arc::new_in(
            Wrapper {
                inst: ptr.cast::<c_void>(),
            },
            allocator,
        );

        Self {
            ptr: Gc::from(ptr),
            handle,
        }
    }

    /// Converts the current `Gc` instance to a `Gc` instance of another type.
    ///
    /// # Safety
    /// See [`Gc::<T>::up_cast`].
    pub fn up_cast<U>(&self) -> SafePtr<U>
    where
        *mut U: GcType,
        U: for<'a> Type<Held<'a> = Option<&'a mut U>>,
        T: AsMut<U>, // ensures T is convertible to U
    {
        SafePtr {
            ptr: self.ptr.up_cast(),
            handle: Arc::clone(&self.handle),
        }
    }
  
    /// Converts the current `Gc` instance to a `Gc` instance of another type.
    ///
    /// # Safety
    /// See [`Gc::<T>::down_cast`].
    pub fn down_cast<U>(&self) -> Result<SafePtr<U>, String>
    where
        *mut U: GcType,
        U: for<'a> Type<Held<'a> = Option<&'a mut U>>,
        T: ObjectType,
    {
        let downcasted_ptr = self.ptr.down_cast::<U>()?;
        Ok(SafePtr {
            ptr: downcasted_ptr,
            handle: Arc::clone(&self.handle),
        })
    }
}

impl<T> Clone for SafePtr<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            handle: Arc::clone(&self.handle),
        }
    }
}

impl<T> PartialEq for SafePtr<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn eq(&self, other: &Self) -> bool {
        self.ptr == other.ptr
    }
}

impl <T> Eq for SafePtr<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
}

impl<T> Deref for SafePtr<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: `ptr` is rooted for as long as `self.handle` is alive (see
        // `Wrapper`'s doc comment), and was non-null when this `SafePtr` was
        // constructed (`SafePtr::new` asserts it).
        &*self.ptr 
    }
}

impl<T> DerefMut for SafePtr<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: `ptr` is rooted for as long as `self.handle` is alive (see
        // `Wrapper`'s doc comment), and was non-null when this `SafePtr` was
        // constructed (`SafePtr::new` asserts it).
        &mut *self.ptr 
    }
}

impl<T> Debug for SafePtr<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "SafePtr<{}>({:?})", T::CLASS_NAME, self.ptr)
    }
}

impl<T> From<SafePtr<T>> for Gc<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn from(safe_ptr: SafePtr<T>) -> Self {
        safe_ptr.ptr
    }
}

impl<T> From<Gc<T>> for SafePtr<T>
where
    *mut T: GcType,
    T: for<'a> Type<Held<'a> = Option<&'a mut T>>,
{
    fn from(gc: Gc<T>) -> Self {
        Self::new(gc)
    }
}