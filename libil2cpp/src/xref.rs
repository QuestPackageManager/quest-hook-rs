pub mod pattern;

mod module_bytes;

#[cfg(target_arch = "aarch64")]
pub(crate) mod arm64;
#[cfg(target_arch = "aarch64")]
pub(crate) use arm64 as arch;

// #[cfg(not(any(target_arch = "aarch64")))]
// compile_error!("No supported xref architecture for this target");

use std::sync::OnceLock;

use crate::raw::{GcFunctions, IL2CPP_BINARY};

/// Returns the bytes of the running `libil2cpp`/`GameAssembly` binary, as
/// currently mapped in this process's own memory - used for pattern
/// scanning and branch tracing against the live, relocated code image (as
/// opposed to the on-disk file, which isn't what's actually executed).
///
/// # Panics
/// Panics if the module can't be located in this process's memory - this
/// should only happen if `libil2cpp`/`GameAssembly` somehow isn't loaded
/// yet, which would mean every other xref lookup is also unusable.
fn get_il2cpp_bytes() -> &'static [u8] {
    static BYTES: OnceLock<&'static [u8]> = OnceLock::new();
    *BYTES.get_or_init(|| {
        unsafe { module_bytes::find(IL2CPP_BINARY) }
            .unwrap_or_else(|| panic!("could not locate {IL2CPP_BINARY} in this process's memory"))
    })
}

/// Initialize xref-based GC function resolution for libil2cpp. This should
/// be called once at the start of the program, before any other libil2cpp
/// functions that need GC allocation (e.g. [`crate::GcAllocator`]) are used.
pub fn xref_init() {
    GcFunctions::init(get_il2cpp_bytes());
}
