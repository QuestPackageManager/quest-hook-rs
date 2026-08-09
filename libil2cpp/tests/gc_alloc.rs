//! Integration test against a real fixture binary under
//! `tests/il2cpp_v31/`, exercising only `libil2cpp`'s public API -
//! `xref_init` and `GcAllocator` - the same way an actual consumer of the
//! crate would.
//!
//! `dlopen`/`LoadLibrary` refuse to load a shared object built for a
//! different (os, arch) than the process loading it, so this relies on
//! `.cargo/config.toml` pointing this target's `runner` at
//! `run_with_fixture.*`, which puts the fixture directory matching the
//! target this test was compiled for on the dynamic loader's search path
//! *before* this process starts (required - `dlopen`/`LoadLibrary` won't
//! see a search-path env var set after the fact by the process itself).
//! There's no graceful skip for a target without a fixture/runner set up
//! yet - `ensure_fixture_present` asserts it's there, so this fails loudly
//! (rather than silently passing) as a reminder to add one.
#![cfg(feature = "xref")]

use std::path::PathBuf;
use std::sync::LazyLock;

use libil2cpp::raw::{IL2CPP_BINARY, LIBIL2CPP};
use libil2cpp::GcAllocator;

/// `tests/il2cpp_v31/<FIXTURE_DIR>/<IL2CPP_BINARY>` for whatever (os, arch) -
/// must match the fixture directory name passed to `run_with_fixture.*` in
/// `.cargo/config.toml` for this target.
#[cfg(all(target_os = "android", target_arch = "aarch64"))]
const FIXTURE_DIR: &str = "android-aarch64";
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const FIXTURE_DIR: &str = "linux-x64";
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const FIXTURE_DIR: &str = "windows-x64";
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const FIXTURE_DIR: &str = "macos-x86_64";
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const FIXTURE_DIR: &str = "macos-aarch64";

/// Asserts the fixture binary for this target is actually checked in - a
/// missing fixture for a target declared in `FIXTURE_DIR` above is a setup
/// bug, not something to skip quietly past.
fn ensure_fixture_present() {
    let fixture_dir: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "il2cpp_v31",
        FIXTURE_DIR,
    ]
    .iter()
    .collect();
    let binary_path = fixture_dir.join(IL2CPP_BINARY);
    assert!(
        binary_path.is_file(),
        "no il2cpp_v31 fixture at {} (target: {}-{}) - add one, or add a \
         FIXTURE_DIR case above if this target isn't meant to be supported yet",
        binary_path.display(),
        std::env::consts::OS,
        std::env::consts::ARCH,
    );

    // `xref_init` assumes libil2cpp is already loaded - true on Quest,
    // where the game process loads it long before a mod runs. Force that
    // precondition here too, rather than relying on some other `raw::` call
    // having incidentally triggered `LIBIL2CPP`'s lazy `dlopen` first.
    LazyLock::force(&LIBIL2CPP);
}

#[test]
fn gc_allocator_initializes_against_the_real_fixture() {
    ensure_fixture_present();

    libil2cpp::xref_init();

    // We deliberately stop at construction rather than actually allocating:
    // the real gc_alloc_fixed/gc_free_fixed only work once il2cpp's GC and
    // domain are fully initialized (thread registration, heap setup, ...),
    // which loading the library alone doesn't do - calling them here would
    // be likely to crash rather than exercise anything meaningful. This
    // test's job is just to confirm the public xref_init -> GcAllocator
    // wiring resolves real symbols end to end.
    GcAllocator::new().expect("GcAllocator::new should succeed once xref_init has run");
}
