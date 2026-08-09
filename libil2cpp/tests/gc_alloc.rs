//! Integration test against a real fixture binary under
//! `tests/il2cpp_v31/`, exercising only `libil2cpp`'s public API -
//! `xref_init` and `GcAllocator` - the same way an actual consumer of the
//! crate would.
//!
//! `dlopen`/`LoadLibrary` refuse to load a shared object built for a
//! different (os, arch) than the process loading it, so this looks up
//! whichever fixture directory matches the target this test itself was
//! compiled for. There's no graceful skip for a target without a fixture
//! yet - `ensure_fixture_loadable` asserts it's there, so this fails loudly
//! (rather than silently passing) as a reminder to add one, on any target
//! declared below that doesn't have a fixture checked in.
#![cfg(feature = "xref")]

use std::path::PathBuf;

use libil2cpp::raw::IL2CPP_BINARY;
use libil2cpp::GcAllocator;

/// `tests/il2cpp_v31/<FIXTURE_DIR>/<IL2CPP_BINARY>` for whatever (os, arch)
#[cfg(all(target_os = "android", target_arch = "aarch64"))]
const FIXTURE_DIR: &str = "android-aarch64";
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
const FIXTURE_DIR: &str = "linux-x86_64";
#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
const FIXTURE_DIR: &str = "windows-x86_64";
#[cfg(all(target_os = "macos", target_arch = "x86_64"))]
const FIXTURE_DIR: &str = "macos-x86_64";
#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
const FIXTURE_DIR: &str = "macos-aarch64";

/// The env var the dynamic loader reads to find libraries referenced by
/// bare name, like `IL2CPP_BINARY`.
#[cfg(target_os = "windows")]
const LIBRARY_SEARCH_PATH_VAR: &str = "PATH";
#[cfg(target_os = "macos")]
const LIBRARY_SEARCH_PATH_VAR: &str = "DYLD_LIBRARY_PATH";
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const LIBRARY_SEARCH_PATH_VAR: &str = "LD_LIBRARY_PATH";

#[cfg(target_os = "windows")]
const SEARCH_PATH_SEPARATOR: char = ';';
#[cfg(not(target_os = "windows"))]
const SEARCH_PATH_SEPARATOR: char = ':';

fn prepend_to_search_path(dir: &std::path::Path) {
    let existing = std::env::var(LIBRARY_SEARCH_PATH_VAR).unwrap_or_default();
    let new_value = if existing.is_empty() {
        dir.display().to_string()
    } else {
        format!("{}{SEARCH_PATH_SEPARATOR}{existing}", dir.display())
    };
    unsafe { std::env::set_var(LIBRARY_SEARCH_PATH_VAR, new_value) };
}

/// Points the dynamic loader's search path at this target's fixture
/// directory. Panics if it isn't there - a missing fixture for a target
/// declared in `FIXTURE_DIR` above is a setup bug, not something to skip
/// quietly past.
fn ensure_fixture_loadable() {
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

    prepend_to_search_path(&fixture_dir);
}

#[test]
fn gc_allocator_initializes_against_the_real_fixture() {
    ensure_fixture_loadable();

    libil2cpp::xref_init(&[]);

    // We deliberately stop at construction rather than actually allocating:
    // the real gc_alloc_fixed/gc_free_fixed only work once il2cpp's GC and
    // domain are fully initialized (thread registration, heap setup, ...),
    // which loading the library alone doesn't do - calling them here would
    // be likely to crash rather than exercise anything meaningful. This
    // test's job is just to confirm the public xref_init -> GcAllocator
    // wiring resolves real symbols end to end.
    GcAllocator::new().expect("GcAllocator::new should succeed once xref_init has run");
}
