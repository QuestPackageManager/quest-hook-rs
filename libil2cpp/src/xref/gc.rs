use std::ffi::c_void;

use crate::xref::arch::gc::{find_gc_alloc_fixed, find_gc_free, find_gc_free_fixed};

pub type GcAllocFixedFn = unsafe extern "C" fn(size: usize) -> *mut c_void;
pub type GcFreeFn = unsafe extern "C" fn(obj: *mut c_void);
pub type GcFreeFixedFn = unsafe extern "C" fn(obj: *mut c_void);

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
