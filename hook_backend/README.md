# `hook_backend`

A cross-platform inline function hooking abstraction used internally by `quest_hook`.

The `Hook` implementation is chosen at compile time via Cargo features, so exactly one backend feature must be enabled for the target you're building:

| Feature       | Platform                | Backend |
|---------------|--------------------------|---------|
| `inline_hook` | Android AArch64 / ARMv7  | Vendored `And64InlineHook` / `inlineHook.c` |
| `flamingo`    | Android AArch64          | [`flamingo_rs`](https://github.com/QuestPackageManager/flamingo_rs) |
| `retour`      | Windows / Linux / macOS  | [`retour`](https://crates.io/crates/retour) `RawDetour` |

`inline_hook` and `flamingo` are mutually exclusive on AArch64 Android; enabling both is a compile error. Neither Android backend feature is required on non-Android targets, but `retour` must be enabled there.

## Cargo.toml

```toml
[dependencies]
hook_backend = { git = "https://github.com/StackDoubleFlow/quest-hook-rs.git", features = ["inline_hook", "retour"] }
```

> Most users should enable the `inline_hook` / `flamingo` feature on `quest_hook` instead of depending on this crate directly. The `#[hook]` macro wires everything up automatically.

## API

```rust
use hook_backend::Hook;

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

## `flamingo` build requirement

The `flamingo` feature uses [QPM](https://github.com/QuestPackageManager/QPM.CLI) to restore its native dependency (`libflamingo.so`) at build time. QPM must be installed and available on `PATH` (or pointed to via the `QPM_PATH` environment variable) before building for Android with this feature.
