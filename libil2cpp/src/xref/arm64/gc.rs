use std::slice;

use crate::xref::arch::disasm::MAX_BRANCH_SEARCH_INSTRUCTIONS;

/// Reads `MAX_BRANCH_SEARCH_INSTRUCTIONS` worth of code starting at `addr` in
/// this process's own memory (as opposed to the on-disk `libil2cpp` buffer),
/// for disassembling from a live, dynamically-resolved symbol.
///
/// # Safety
/// `addr` must be a valid pointer into mapped, readable executable memory
/// (e.g. a resolved libil2cpp function address).
unsafe fn live_code_at(addr: usize) -> &'static [u8] {
    unsafe { slice::from_raw_parts(addr as *const u8, MAX_BRANCH_SEARCH_INSTRUCTIONS * 4) }
}

/*
 * XREF for GarbageCollector::AllocateFixed (unity2018, il2cpp_v24)
 * - il2cpp_domain_get (XREF_FOUND, exported symbol)
 *   - B<1> -> Domain::GetCurrent
 *     - BL<1> -> GarbageCollector::AllocateFixed (2-arg: size, descr -
 *       different signature than the `il2cpp_gc_alloc_fixed` exported on
 *       newer versions; beatsaber-hook always passes descr = null, so we do
 *       too and expose the same 1-arg shape)
 * - fallback if the above isn't found: GC_Malloc_Uncollectable (XREF_FOUND,
 *   AOB pattern - hardcodes type id 2, see wrapper_gc_malloc_uncollectable)
 *
 * https://github.com/QuestPackageManager/beatsaber-hook/blob/7632eb7bf2634dabbf3cade1df140e5d93f48845/src/api.cpp#L530-L566
 */
#[cfg(any(feature = "unity2018", feature = "il2cpp_v24"))]
mod legacy_alloc_fixed {
    use std::ptr;
    use std::sync::OnceLock;

    use super::live_code_at;
    use crate::raw;
    use crate::xref::arch::disasm::{
        find_nth_bl_target, find_nth_branch_target, MAX_BRANCH_SEARCH_INSTRUCTIONS,
    };
    use crate::raw::GcAllocFixedFn;
    use crate::xref::pattern::find_unique_pattern;
    use std::ffi::c_void;

    // https://github.com/QuestPackageManager/beatsaber-hook/blob/master/src/api.cpp#L844-L848
    // `Domain::GetCurrent` isn't exported, so we find it as the first branch
    // `il2cpp_domain_get` (which *is* exported, and resolvable by symbol
    // name) makes.
    fn find_domain_get_current() -> Option<usize> {
        let domain_get = raw::symbol_addr(b"il2cpp_domain_get")?;
        let code = unsafe { live_code_at(domain_get) };

        find_nth_branch_target(code, domain_get, 1, MAX_BRANCH_SEARCH_INSTRUCTIONS)
    }

    // `Domain::GetCurrent` has a single `bl` to `GarbageCollector::AllocateFixed`.
    fn trace_gc_alloc_fixed(domain_get_current: usize) -> Option<usize> {
        let code = unsafe { live_code_at(domain_get_current) };

        find_nth_bl_target(code, domain_get_current, 1, MAX_BRANCH_SEARCH_INSTRUCTIONS)
    }

    /// `GarbageCollector::AllocateFixed`'s actual signature - unlike the
    /// directly-exported `il2cpp_gc_alloc_fixed`, this internal function
    /// takes a type descriptor. beatsaber-hook's own callers always pass
    /// null for it, so we do the same and expose the same `(size)` shape as
    /// the newer exported symbol.
    type GarbageCollectorAllocateFixedFn =
        unsafe extern "C" fn(size: usize, descr: *mut c_void) -> *mut c_void;

    static GARBAGE_COLLECTOR_ALLOCATE_FIXED: OnceLock<GarbageCollectorAllocateFixedFn> =
        OnceLock::new();

    unsafe extern "C" fn wrapper_garbage_collector_allocate_fixed(size: usize) -> *mut c_void {
        let wrapped = GARBAGE_COLLECTOR_ALLOCATE_FIXED.get().expect(
            "wrapper_garbage_collector_allocate_fixed called before find_gc_alloc_fixed resolved it",
        );
        unsafe { wrapped(size, ptr::null_mut()) }
    }

    /// `GC_Malloc_Uncollectable`'s actual signature - it takes a GC type id
    /// rather than a type descriptor.
    type GcMallocUncollectableFn = unsafe extern "C" fn(size: usize, type_id: i64) -> *mut c_void;

    static WRAPPED_GC_MALLOC_UNCOLLECTABLE: OnceLock<GcMallocUncollectableFn> = OnceLock::new();

    /// Adapts `GC_Malloc_Uncollectable`'s `(size, type_id)` signature to the
    /// `(size)` shape `GcAllocFixedFn` callers expect, hardcoding the type id
    /// beatsaber-hook determined from a Ghidra dump of a caller (`2`, i.e.
    /// `GC_signature_words`/pointer-free uncollectable data).
    unsafe extern "C" fn wrapper_gc_malloc_uncollectable(size: usize) -> *mut c_void {
        let wrapped = WRAPPED_GC_MALLOC_UNCOLLECTABLE.get().expect(
            "wrapper_gc_malloc_uncollectable called before find_gc_alloc_fixed resolved it",
        );
        unsafe { wrapped(size, 2) }
    }

    pub fn find_gc_alloc_fixed(libil2cpp: &[u8]) -> Option<GcAllocFixedFn> {
        if let Some(target) = find_domain_get_current().and_then(trace_gc_alloc_fixed) {
            GARBAGE_COLLECTOR_ALLOCATE_FIXED
                .set(unsafe {
                    std::mem::transmute::<usize, GarbageCollectorAllocateFixedFn>(target)
                })
                .ok()?;
            return Some(wrapper_garbage_collector_allocate_fixed);
        }

        // `Domain::GetCurrent`'s branch to `GarbageCollector::AllocateFixed`
        // wasn't found - fall back to signature-scanning
        // `GC_Malloc_Uncollectable` and wrapping it into the shape
        // `GcAllocFixedFn` expects.
        let addr = find_unique_pattern(
            libil2cpp,
            "f5 0f 1d f8 f4 4f 01 a9 fd 7b 02 a9 fd 83 00 91 ?? ?? ?? ?? ?? ?? ?? ?? 1f 00 20 f1 f3 03 01 2a",
            "GC_Malloc_Uncollectable",
        )
        .ok()?;

        WRAPPED_GC_MALLOC_UNCOLLECTABLE
            .set(unsafe { std::mem::transmute::<usize, GcMallocUncollectableFn>(addr) })
            .ok()?;

        Some(wrapper_gc_malloc_uncollectable)
    }
}

#[cfg(any(feature = "unity2018", feature = "il2cpp_v24"))]
pub use legacy_alloc_fixed::find_gc_alloc_fixed;
