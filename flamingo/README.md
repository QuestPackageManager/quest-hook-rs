# `flamingo`

Rust bindings for [Flamingo](https://github.com/QuestPackageManager/Flamingo), the native inline-hooking library used on the Meta Quest / Android platform.

On Android (AArch64) this crate wraps the `libflamingo.so` native library via a generated C API. On all other platforms it falls back to the [`retour`](https://crates.io/crates/retour) crate, matching the behaviour of `inline_hook`.

## Cargo.toml

```toml
[dependencies]
quest_hook = { git = "https://github.com/StackDoubleFlow/quest-hook-rs.git", features = ["flamingo"] }
```

Or, to depend on this crate directly:

```toml
[dependencies]
flamingo = { git = "https://github.com/StackDoubleFlow/quest-hook-rs.git" }
```

## Build requirements

This crate uses [QPM](https://github.com/QuestPackageManager/QPM.CLI) to restore its native dependency (`libflamingo.so`) at build time. QPM must be installed and available on `PATH` (or pointed to via the `QPM_PATH` environment variable) before building for Android.

```
qpm restore   # run once in the flamingo/ directory to fetch libflamingo.so
```

The binding header (`wrapper.h`) and `libflamingo.so` are placed under `flamingo/extern/` after restoration. `build.rs` invokes `bindgen` to generate the Rust FFI bindings at compile time.

## API

The public API mirrors `inline_hook`:

```rust
use flamingo::Hook;

static MY_HOOK: Hook = Hook::new();

unsafe {
    MY_HOOK.install(target_fn as *const (), replacement_fn as *const ());
}

if let Some(original) = MY_HOOK.original() {
    let original_fn: fn(u32) -> u32 = std::mem::transmute(original);
    let _ = original_fn(42);
}
```

### `Hook`

| Method | Description |
|--------|-------------|
| `Hook::new()` | Create an uninitialised hook (safe in `static` context) |
| `unsafe install(target, hook) -> bool` | Redirect `target` to `hook` |
| `is_installed() -> bool` | Whether the hook is currently active |
| `original() -> Option<*const ()>` | Trampoline pointer to the original function |

## Platform matrix

| Platform | Backend |
|----------|---------|
| Android AArch64 | `libflamingo.so` via `flamingo_c_api` (bindgen) |
| All others | [`retour`](https://crates.io/crates/retour) `RawDetour` |

## QPM dependency

| Package | Version |
|---------|---------|
| `flamingo` | `^1.2.1` |

The `.so` is downloaded from the [Flamingo GitHub releases](https://github.com/QuestPackageManager/Flamingo/releases).
