use std::ffi::c_void;
use std::slice;

use crate::raw;
use crate::xref::arch::disasm::{find_nth_branch_target, MAX_BRANCH_SEARCH_INSTRUCTIONS};
use crate::xref::gc::{GcAllocFixedFn, GcFreeFixedFn, GcFreeFn};

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
 * XREF for GarbageCollector::AllocateFixed (il2cpp_v29, il2cpp_v31)
 * - il2cpp_gc_alloc_fixed (XREF_FOUND, exported symbol)
 *
 * https://github.com/QuestPackageManager/beatsaber-hook/blob/master/src/api.cpp#L142
 */
#[cfg(any(feature = "il2cpp_v29", feature = "il2cpp_v31"))]
pub fn find_gc_alloc_fixed(_libil2cpp: &[u8]) -> Option<GcAllocFixedFn> {
    raw::symbol_addr(b"il2cpp_gc_alloc_fixed")
        .map(|addr| unsafe { std::mem::transmute::<usize, GcAllocFixedFn>(addr) })
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
    use crate::xref::gc::GcAllocFixedFn;
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

/*
 * XREF for GC_free
 * - il2cpp_gc_free_fixed (XREF_FOUND, exported symbol - see
 *   find_gc_free_fixed)
 *   - B<1> -> GC_free
 *
 * https://github.com/QuestPackageManager/beatsaber-hook/blob/7632eb7bf2634dabbf3cade1df140e5d93f48845/src/utils/utils.cpp#L26-L34
 */
pub fn find_gc_free(gc_free_fixed: GcFreeFixedFn) -> Option<GcFreeFn> {
    let addr = gc_free_fixed as usize;
    let code = unsafe { live_code_at(addr) };

    find_nth_branch_target(code, addr, 1, MAX_BRANCH_SEARCH_INSTRUCTIONS)
        .map(|target| unsafe { std::mem::transmute::<usize, GcFreeFn>(target) })
}

/*
 * XREF for GarbageCollector::FreeFixed
 * - il2cpp_gc_free_fixed (XREF_FOUND, exported symbol)
 *
 * https://github.com/QuestPackageManager/beatsaber-hook/blob/master/src/api.cpp#L890
 */
pub fn find_gc_free_fixed() -> Option<GcFreeFixedFn> {
    raw::symbol_addr(b"il2cpp_gc_free_fixed")
        .map(|addr| unsafe { std::mem::transmute::<usize, GcFreeFixedFn>(addr) })
}

#[cfg(test)]
mod tests {
    use super::*;

    // `il2cpp_gc_free_fixed` (vaddr 0x9a979c): a single `b 0x9a3888`. Bytes
    // and addresses verified via `llvm-objdump -d` against
    // `libil2cpp/tests/il2cpp_v31/android/libil2cpp.so`.
    const GC_FREE_FIXED_BYTES: [u8; 4] = [0x3b, 0xe8, 0xff, 0x17];
    const GC_FREE_FIXED_ADDR: usize = 0x9a979c;
    const GC_FREE_FIXED_BRANCH_TARGET: usize = 0x9a3888;

    #[test]
    fn find_gc_free_follows_gc_free_fixeds_branch() {
        // `live_code_at` always reads a fixed `MAX_BRANCH_SEARCH_INSTRUCTIONS
        // * 4`-byte window, so the backing buffer must be that big (even
        // though only the leading bytes are real instructions) or the read
        // runs past the buffer.
        let mut buf = Box::new([0u8; MAX_BRANCH_SEARCH_INSTRUCTIONS * 4]);
        buf[..GC_FREE_FIXED_BYTES.len()].copy_from_slice(&GC_FREE_FIXED_BYTES);

        let fn_addr = buf.as_ptr() as usize;
        let gc_free_fixed: GcFreeFixedFn =
            unsafe { std::mem::transmute::<usize, GcFreeFixedFn>(fn_addr) };

        let gc_free = find_gc_free(gc_free_fixed).expect("should follow gc_free_fixed's branch");

        // The branch is PC-relative, so this delta holds regardless of
        // where our buffer actually lives in memory.
        let expected_delta = GC_FREE_FIXED_BRANCH_TARGET as i64 - GC_FREE_FIXED_ADDR as i64;
        assert_eq!(gc_free as usize as i64 - fn_addr as i64, expected_delta);
    }

    // End-to-end check against the real fixture binary: points the dynamic
    // loader's search path at the fixture directory matching this test
    // binary's own (necessarily aarch64) target, then exercises the actual
    // symbol-resolution path (not just the pure disassembly math above).
    // This module is arm64-specific, so unlike `tests/gc_alloc.rs` there's
    // no non-aarch64 case to handle - only the OS varies.
    #[cfg(feature = "il2cpp_v31")]
    mod fixture {
        use std::path::PathBuf;

        use super::*;

        // Pre-existing fixture, kept as "android" rather than
        // "android-aarch64" for continuity - Android is effectively always
        // this project's only aarch64 target so far. Add a same-shaped
        // `#[cfg]` variant (and the fixture file itself) for a new aarch64
        // OS target.
        #[cfg(target_os = "android")]
        const FIXTURE_DIR: &str = "android";
        #[cfg(target_os = "linux")]
        const FIXTURE_DIR: &str = "linux-aarch64";
        #[cfg(target_os = "macos")]
        const FIXTURE_DIR: &str = "macos-aarch64";
        #[cfg(target_os = "windows")]
        const FIXTURE_DIR: &str = "windows-aarch64";

        /// Points the dynamic loader's search path at this target's fixture
        /// directory. Panics if it isn't there - a missing fixture for a
        /// target declared in `FIXTURE_DIR` above is a setup bug, not
        /// something to skip quietly past.
        fn ensure_fixture_loadable() {
            let fixture_dir: PathBuf = [
                env!("CARGO_MANIFEST_DIR"),
                "tests",
                "il2cpp_v31",
                FIXTURE_DIR,
            ]
            .iter()
            .collect();
            let binary_path = fixture_dir.join(raw::IL2CPP_BINARY);
            assert!(
                binary_path.is_file(),
                "no il2cpp_v31 fixture at {} (target: {}-{}) - add one, or add a \
                 FIXTURE_DIR case above if this target isn't meant to be \
                 supported yet",
                binary_path.display(),
                std::env::consts::OS,
                std::env::consts::ARCH,
            );

            let existing = std::env::var("LD_LIBRARY_PATH").unwrap_or_default();
            let new_path = if existing.is_empty() {
                fixture_dir.display().to_string()
            } else {
                format!("{}:{existing}", fixture_dir.display())
            };
            unsafe { std::env::set_var("LD_LIBRARY_PATH", new_path) };
        }

        #[test]
        fn resolves_gc_free_fixed_and_gc_free_against_the_real_fixture() {
            ensure_fixture_loadable();

            let gc_free_fixed = find_gc_free_fixed().expect("il2cpp_gc_free_fixed should resolve");
            let gc_alloc_fixed_addr = raw::symbol_addr(b"il2cpp_gc_alloc_fixed")
                .expect("il2cpp_gc_alloc_fixed should resolve");

            // Known-good deltas between real, objdump-verified addresses -
            // invariant under ASLR, since both symbols live in the same
            // loaded module and shift together.
            assert_eq!(
                gc_free_fixed as usize as i64 - gc_alloc_fixed_addr as i64,
                0x9a979c - 0x9a9794
            );

            let gc_free =
                find_gc_free(gc_free_fixed).expect("should follow gc_free_fixed's branch");
            assert_eq!(
                gc_free as usize as i64 - gc_free_fixed as usize as i64,
                0x9a3888 - 0x9a979c
            );
        }

        #[test]
        fn resolves_gc_alloc_fixed_via_the_exported_symbol() {
            ensure_fixture_loadable();

            let gc_alloc_fixed =
                find_gc_alloc_fixed(&[]).expect("il2cpp_gc_alloc_fixed should resolve");
            let addr =
                raw::symbol_addr(b"il2cpp_gc_alloc_fixed").expect("symbol_addr should agree");
            assert_eq!(gc_alloc_fixed as usize, addr);
        }
    }
}
