use std::alloc::{AllocError, Allocator, Layout};
use std::ffi::c_void;
use std::ptr::NonNull;

use crate::raw::{GcAllocFixedFn, GcFreeFixedFn, GcFunctions};

/// An allocator that uses GC functions to allocate memory.
/// This is useful for allocating memory that will be managed by the GC, such as
/// objects that will be used in the Unity engine.
#[derive(Clone, Copy)]
pub struct GcAllocator {
    gc_alloc_fixed: GcAllocFixedFn,
    gc_free_fixed: GcFreeFixedFn,
}

impl GcAllocator {
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

pub type GcBox<T> = Box<T, GcAllocator>;
pub type GcVec<T> = Vec<T, GcAllocator>;
pub type GcHashMap<K, V> = std::collections::HashMap<K, V, GcAllocator>;
pub type GcRc<T> = std::rc::Rc<T, GcAllocator>;
pub type GcArc<T> = std::sync::Arc<T, GcAllocator>;
