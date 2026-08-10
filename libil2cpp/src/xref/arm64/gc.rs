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
    use crate::raw::GcAllocFixedFn;
    use crate::xref::arch::disasm::{
        find_nth_bl_target, find_nth_branch_target, MAX_BRANCH_SEARCH_INSTRUCTIONS,
    };
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

    /// Signature-scans `libil2cpp` for `GC_Malloc_Uncollectable`, returning
    /// it as a callable function pointer - resolved from `libil2cpp`'s own
    /// base address plus the pattern's offset into it, *not* the bare offset
    /// `find_unique_pattern` returns.
    fn find_gc_malloc_uncollectable(libil2cpp: &[u8]) -> Option<GcMallocUncollectableFn> {
        let offset = find_unique_pattern(
            libil2cpp,
            "f5 0f 1d f8 f4 4f 01 a9 fd 7b 02 a9 fd 83 00 91 ?? ?? ?? ?? ?? ?? ?? ?? 1f 00 20 f1 f3 03 01 2a",
            "GC_Malloc_Uncollectable",
        )
        .ok()?;

        let addr = libil2cpp.as_ptr() as usize + offset;
        Some(unsafe { std::mem::transmute::<usize, GcMallocUncollectableFn>(addr) })
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
        let found = find_gc_malloc_uncollectable(libil2cpp)?;
        WRAPPED_GC_MALLOC_UNCOLLECTABLE.set(found).ok()?;

        Some(wrapper_gc_malloc_uncollectable)
    }

    #[cfg(test)]
    mod tests {
        use super::find_gc_malloc_uncollectable;

        /// The `GC_Malloc_Uncollectable` AOB pattern, with its wildcard
        /// bytes filled in arbitrarily (they should still match, since
        /// they're wildcards).
        const PATTERN: [u8; 32] = [
            0xf5, 0x0f, 0x1d, 0xf8, 0xf4, 0x4f, 0x01, 0xa9, 0xfd, 0x7b, 0x02, 0xa9, 0xfd, 0x83,
            0x00, 0x91, 0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x11, 0x22, 0x1f, 0x00, 0x20, 0xf1,
            0xf3, 0x03, 0x01, 0x2a,
        ];

        #[test]
        fn resolves_the_patterns_absolute_address_not_its_offset() {
            // Pad with unrelated bytes before the pattern, so a correct
            // implementation has to add the match offset to the haystack's
            // base address, rather than treating the offset as if it *were*
            // the address - the bug this test guards against (it used to
            // hand the bare offset straight to `mem::transmute` as a
            // function pointer).
            let mut haystack = vec![0u8; 96];
            let pattern_offset = 40;
            haystack[pattern_offset..pattern_offset + PATTERN.len()].copy_from_slice(&PATTERN);

            let resolved = find_gc_malloc_uncollectable(&haystack)
                .expect("the pattern was placed in the haystack, so this should match");

            assert_eq!(
                resolved,
                haystack.as_ptr() as usize + pattern_offset,
                "resolved address should be the pattern's absolute address in memory, not its bare offset into the haystack"
            );
        }

        #[test]
        fn returns_none_when_the_pattern_is_absent() {
            let haystack = vec![0u8; 64];
            assert!(find_gc_malloc_uncollectable(&haystack).is_none());
        }
    }
}

#[cfg(any(feature = "unity2018", feature = "il2cpp_v24"))]
pub use legacy_alloc_fixed::find_gc_alloc_fixed;
