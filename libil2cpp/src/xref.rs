pub mod pattern;

#[cfg(target_arch = "aarch64")]
pub(crate) mod arm64;
#[cfg(target_arch = "aarch64")]
pub(crate) use arm64 as arch;

#[cfg(not(any(target_arch = "aarch64")))]
compile_error!("No supported xref architecture for this target");

use crate::raw::GcFunctions;

/// Initialize xref-based GC function resolution for libil2cpp. This should
/// be called once at the start of the program, before any other libil2cpp
/// functions that need GC allocation (e.g. [`crate::GcAllocator`]) are used.
pub fn xref_init(libil2cpp: &[u8]) {
    GcFunctions::init(libil2cpp);
}