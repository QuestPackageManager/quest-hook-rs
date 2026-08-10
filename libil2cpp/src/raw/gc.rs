//! Resolution of the Boehm GC functions used internally by libil2cpp.
//!
//! [`super::gc_alloc_fixed`]/[`super::gc_free_fixed`] (declared alongside
//! the rest of libil2cpp's exported functions in `functions.rs`) are
//! exported symbols, resolved and cached the same way as every other
//! `il2cpp_` function in this crate - no disassembly needed.
//! `GarbageCollector::AllocateFixed` isn't exported on unity2018/il2cpp_v24
//! though, so there it falls back to the deeper, disassembly-based lookup
//! in [`crate::xref`], available only when the `xref` feature is enabled.

use std::ffi::c_void;

#[cfg(any(feature = "il2cpp_v29", feature = "il2cpp_v31"))]
use super::gc_alloc_fixed;
use super::gc_free_fixed;

/// GcAllocFixed allocates a fixed-size object in the Boehm GC heap. The object
/// is not movable and will not be collected until it is explicitly freed.
pub type GcAllocFixedFn = unsafe extern "C" fn(size: usize) -> *mut c_void;
/// GcFreeFixed frees a fixed-size object allocated with GcAllocFixed. The
/// object must not be used after it is freed.
pub type GcFreeFixedFn = unsafe extern "C" fn(obj: *mut c_void);

/// `il2cpp_functions!` gives us a safe, `OnceLock`-cached Rust-ABI wrapper,
/// but [`GcFunctions`] stores plain `extern "C"` pointers so `GcAllocator`
/// can call xref-resolved and exported-symbol functions the same way. This
/// is the thin shim that bridges the two for `gc_alloc_fixed`.
#[cfg(any(feature = "il2cpp_v29", feature = "il2cpp_v31"))]
unsafe extern "C" fn gc_alloc_fixed_shim(size: usize) -> *mut c_void {
    unsafe { gc_alloc_fixed(size) }
}

/// Same as [`gc_alloc_fixed_shim`], for `gc_free_fixed`.
unsafe extern "C" fn gc_free_fixed_shim(obj: *mut c_void) {
    unsafe { gc_free_fixed(obj) };
}

/// Boehm GC functions used by libil2cpp. `gc_alloc_fixed` is `None` if it
/// couldn't be resolved (only possible on unity2018/il2cpp_v24 without the
/// `xref` feature).
pub struct GcFunctions {
    /// `GarbageCollector::AllocateFixed` (or equivalent) function pointer.
    pub gc_alloc_fixed: Option<GcAllocFixedFn>,
    /// `GarbageCollector::FreeFixed` (or equivalent) function pointer.
    pub gc_free_fixed: Option<GcFreeFixedFn>,
}
static GC_FUNCTIONS: std::sync::OnceLock<GcFunctions> = std::sync::OnceLock::new();

impl GcFunctions {
    #[allow(unused_variables, unused_mut, unused_assignments)]
    fn resolve(libil2cpp: &[u8]) -> Self {
        let mut gc_alloc_fixed = None;
        #[cfg(any(feature = "il2cpp_v29", feature = "il2cpp_v31"))]
        {
            gc_alloc_fixed = Some(gc_alloc_fixed_shim as GcAllocFixedFn);
        }
        // unity2018/il2cpp_v24 don't export `GarbageCollector::AllocateFixed`
        // directly - it can only be found by tracing branches (or, failing
        // that, signature-scanning), which needs the `xref` feature.
        #[cfg(all(any(feature = "unity2018", feature = "il2cpp_v24"), feature = "xref"))]
        {
            gc_alloc_fixed = crate::xref::arch::gc::find_gc_alloc_fixed(libil2cpp);
        }

        let gc_free_fixed = Some(gc_free_fixed_shim as GcFreeFixedFn);

        Self {
            gc_alloc_fixed,
            gc_free_fixed,
        }
    }

    /// Resolve and cache the [`GcFunctions`] instance for the given libil2cpp
    /// binary. This should be called once at the start of the program,
    /// before any other libil2
    pub fn init(libil2cpp: &[u8]) -> &'static Self {
        GC_FUNCTIONS.get_or_init(|| Self::resolve(libil2cpp))
    }

    /// Get the cached [`GcFunctions`] instance, if it has been initialized.
    pub fn get() -> Option<&'static Self> {
        GC_FUNCTIONS.get()
    }
}
