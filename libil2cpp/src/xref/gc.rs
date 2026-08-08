use std::ffi::c_void;

use crate::xref::arch::gc::{find_gc_alloc_fixed, find_gc_free, find_gc_free_fixed};

/// GcAllocFixed allocates a fixed-size object in the Boehm GC heap. The object is not movable and will not be collected until it is explicitly freed.
pub type GcAllocFixedFn = unsafe extern "C" fn(size: usize) -> *mut c_void;
/// GcFree frees a fixed-size object allocated with GcAllocFixed. The object must not be used after it is freed.
pub type GcFreeFn = unsafe extern "C" fn(obj: *mut c_void);
/// GcFreeFixed frees a fixed-size object allocated with GcAllocFixed. The object must not be used after it is freed.
pub type GcFreeFixedFn = unsafe extern "C" fn(obj: *mut c_void);

/// Boehm GC functions that are used by libil2cpp. 
pub struct GcFunctions {
    pub gc_alloc_fixed: GcAllocFixedFn,
    pub gc_free: GcFreeFn,
    pub gc_free_fixed: GcFreeFixedFn,
}
static GC_FUNCTIONS: std::sync::OnceLock<GcFunctions> = std::sync::OnceLock::new();

impl GcFunctions {
    fn xref(libil2cpp: &[u8]) -> Result<Self, ()> {
        let gc_alloc_fixed = find_gc_alloc_fixed(libil2cpp).ok_or(())?;
        let gc_free_fixed = find_gc_free_fixed().ok_or(())?;
        let gc_free = find_gc_free(gc_free_fixed).ok_or(())?;

        Ok(Self {
            gc_alloc_fixed,
            gc_free,
            gc_free_fixed,
        })
    }

    pub fn init(libil2cpp: &[u8]) -> Result<&'static Self, ()> {
        GC_FUNCTIONS.get_or_try_init(|| Self::xref(libil2cpp))
    }

    pub fn get() -> Option<&'static Self> {
        GC_FUNCTIONS.get()
    }
}
