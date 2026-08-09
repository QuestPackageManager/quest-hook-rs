#![feature(allocator_api)]
use quest_hook::libil2cpp::{GcAllocator, GcBox, GcVec};
use tracing::debug;

#[no_mangle]
pub extern "C" fn setup() {
    quest_hook::setup_log("gc alloc");
}

#[no_mangle]
pub extern "C" fn load() {
    // Resolves the GC allocation functions from the loaded libil2cpp -
    // must run before `GcAllocator::new()` will succeed.
    quest_hook::libil2cpp::xref_init();

    // `GcBox`/`GcVec` (also `GcRc`, `GcArc`, `GcHashMap`) are just
    // `Box`/`Vec`/... parameterized with `GcAllocator`, so anything that
    // needs to live on the GC heap (e.g. because native code retains a
    // pointer to it past this call) can use the usual std APIs.
    let allocator = GcAllocator::new().expect("GC functions not initialized");
    let boxed: GcBox<u64> = Box::new_in(42, allocator);
    debug!("GC-allocated value: {}", *boxed);

    let mut list: GcVec<u32> = Vec::new_in(allocator);
    list.extend([1, 2, 3]);
    debug!("GC-allocated vec: {list:?}");

    // Both are freed via `gc_free_fixed` when they drop - `boxed` and
    // `list` here, at the end of `load`.
}
