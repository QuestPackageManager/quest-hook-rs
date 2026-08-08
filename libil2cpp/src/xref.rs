pub mod pattern;

#[cfg(target_arch = "aarch64")]
mod arm64;
#[cfg(target_arch = "aarch64")]
use arm64 as arch;

#[cfg(not(any(target_arch = "aarch64")))]
compile_error!("No supported xref architecture for this target");

#[cfg(feature = "gc")]
pub mod gc;

/// Initialize xref for libil2cpp. This function should be called once at the
/// start of the program, before any other libil2cpp functions are used.
pub fn xref_init(libil2cpp: &[u8]) -> Result<(), ()> {
    #[cfg(feature = "gc")]
    gc::GcFunctions::init(libil2cpp)?;
    Ok(())
}
