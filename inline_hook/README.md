# `inline_hook`

A cross-platform inline function hooking abstraction used internally by `quest_hook`.

On Android (AArch64 and ARMv7) this uses a hand-rolled inline hooking implementation. On all other platforms (Windows, Linux, macOS) it delegates to the [`retour`](https://crates.io/crates/retour) crate.

## Cargo.toml

```toml
[dependencies]
inline_hook = { git = "https://github.com/StackDoubleFlow/quest-hook-rs.git" }
```

> Most users should enable the `inline_hook` feature on `quest_hook` instead of depending on this crate directly. The `#[hook]` macro wires everything up automatically.

## API

```rust
use inline_hook::Hook;

// Declare as a static so the hook lives for the lifetime of the process
static MY_HOOK: Hook = Hook::new();

unsafe {
    MY_HOOK.install(target_fn as *const (), replacement_fn as *const ());
}

// Call the original function through the trampoline
if let Some(original) = MY_HOOK.original() {
    let original_fn: fn(u32) -> u32 = std::mem::transmute(original);
    let result = original_fn(42);
}
```

### `Hook`

| Method | Description |
|--------|-------------|
| `Hook::new()` | Create an uninitialised hook (usable in `static` context) |
| `unsafe install(target, hook) -> bool` | Redirect `target` to `hook`; returns `true` on success |
| `is_installed() -> bool` | Whether the hook is currently active |
| `original() -> Option<*const ()>` | Trampoline pointer to the original function, if installed |

## Safety

- `target` and `hook` passed to `install` must have identical signatures and calling conventions.
- The hook is installed for the lifetime of the process; there is currently no `uninstall`.

## Platform matrix

| Platform | Backend |
|----------|---------|
| Android AArch64 | Custom inline hook (`aarch64_linux_android.rs`) |
| Android ARMv7 | Custom inline hook (`armv7_linux_androideabi.rs`) |
| Windows / Linux / macOS | [`retour`](https://crates.io/crates/retour) `RawDetour` |
