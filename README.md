# `quest_hook`

A library for writing (mostly) memory safe mods for Unity il2cpp games

[![Docs](https://img.shields.io/github/workflow/status/StackDoubleFlow/quest-hook-rs/Docs?color=blue&label=docs&style=for-the-badge)](https://stackdoubleflow.github.io/quest-hook-rs/quest_hook/) [![Tests](https://img.shields.io/github/workflow/status/StackDoubleFlow/quest-hook-rs/Tests?label=tests&style=for-the-badge)](https://github.com/StackDoubleFlow/quest-hook-rs/actions/workflows/tests.yml)

## Platform support

Despite its name and initial target and scope, this library supports modding most il2cpp games, as long as you have a way to load the mods.

### il2cpp versions

| Feature flag   | il2cpp version |
|----------------|----------------|
| `il2cpp_v31`   | Unity 2021+    |
| `il2cpp_v29`   | Unity 2019–2020 |
| `il2cpp_v24`   | Unity 2018     |
| `unity2018`    | Unity 2018 (legacy alias) |

Exactly one version feature must be enabled. The default is `il2cpp_v31`.

### Unity versions

- Unity 2019
- Unity 2018

### Targets

| Platform | Architecture |
|----------|-------------|
| Android  | `AArch64` (`ARMv8`) |
| Android  | `ARMv7` |
| Windows  | x86\_64 / x86 |
| Linux    | x86\_64 / x86 |
| macOS    | x86\_64 |

## Quick start

Add `quest_hook` as a dependency and set your crate type to a C dynamic library. A **nightly** Rust toolchain is required.

```toml
# Cargo.toml
[lib]
crate-type = ["cdylib"]

[dependencies]
quest_hook = { git = "https://github.com/StackDoubleFlow/quest-hook-rs.git" }
```

The default feature set (`il2cpp_v31 + util + cache + inline_hook + retour`) is a sensible starting point.
To target an older game, disable the default il2cpp version and enable the correct one:

```toml
[dependencies]
quest_hook = { git = "https://github.com/StackDoubleFlow/quest-hook-rs.git", default-features = false, features = ["il2cpp_v29", "util", "cache", "inline_hook", "retour"] }
```

A nightly toolchain can be pinned project-wide with a `rust-toolchain.toml`:

```toml
# rust-toolchain.toml
[toolchain]
channel = "nightly"
```

## Example

```rust
use quest_hook::hook;
use quest_hook::libil2cpp::{Il2CppObject, Il2CppString};
use tracing::debug;

#[hook("UnityEngine.SceneManagement", "SceneManager", "SetActiveScene")]
fn set_active_scene(scene: &mut Il2CppObject) -> bool {
    let name: &Il2CppString = scene.invoke("get_name", ()).unwrap();
    debug!("Hello, {}!", name);

    set_active_scene.original(scene)
}

#[no_mangle]
pub extern "C" fn setup() {
    quest_hook::setup_log("hello world");
}

#[no_mangle]
pub extern "C" fn load() {
    set_active_scene.install().unwrap();
}
```

See the [`examples/`](./examples/) directory for more complete examples including custom il2cpp types.

## Memory management

Most APIs in this crate hand back a `Gc<T>` — a thin, `Copy`, nullable pointer to a managed (C#) il2cpp object.

`Gc<T>` is intentionally a **weak** reference. IL2CPP uses the Boehm Garbage Collector, the same one as Mono. The garbage collector only scans memory it knows to be a root (the stack, static fields, a handful of explicitly-registered allocations etc.). This implies that memory outside of the stack, e.g the heap, does not get scanned by the GC and will consequently be cleaned up. Additionally, the GC is not aware of newly created threads (unless we tell it), so a `Gc<T>` sitting in an ordinary Rust variable or struct field is invisible to it. This also means that variables in Rust async are not noticed by the GC.

The GC will periodically scan and destroy objects if nothing else keeps it alive. `Gc<T>` is safe to receive, null-check, and use immediately, but not safe to hold onto. Destroyed objects have no indicators that they've been destroyed, and you will reach a SEGFAULT (usually `SEGV_MAPERR`).

See more info here [<https://www.hboehm.info/ismm/04tutorial.pdf>]. 

`NonNullGc<T>` is the same weak pointer with nullability checked out of the type, for when you've already confirmed a `Gc<T>` isn't null and want the compiler to remember that.

`SafePtr<T>` is what actually solves the dangling-pointer problem. It roots the object so it's guaranteed to survive for as long as the `SafePtr<T>` (or any of its clones) exists. It's cheap to `Clone` and safe to hold across collections, store in a struct, or share across threads. Reach for it whenever you need to keep an object alive past the call. It is safe to `Send` and `Sync`, as it is an `Arc` under the hood.

```rust
use quest_hook::libil2cpp::{Gc, Il2CppObject, SafePtr};

// BAD: a `Gc<T>` stashed in a heap-allocated struct is invisible to the GC.
struct Cache {
    target: Gc<Il2CppObject>,
}

impl Cache {
    fn new(target: Gc<Il2CppObject>) -> Box<Self> {
        // `target` is only a root while it sits in this register/stack
        // slot. Once it's copied into `Cache` and `Cache` is boxed onto
        // the heap, the GC has no way to find it. Any collection that
        // runs afterwards - triggered by unrelated allocations elsewhere
        // in the game, possibly on another thread - is free to reclaim
        // the object between this line and whenever `self.target` is
        // next read.
        Box::new(Self { target })
    }
}

// GOOD: root it into a `SafePtr<T>` before it leaves the stack.
struct SafeCache {
    target: SafePtr<Il2CppObject>,
}

impl SafeCache {
    fn new(target: Gc<Il2CppObject>) -> Box<Self> {
        Box::new(Self { target: target.into_safe_ptr() })
    }
}
```

## Type system

Every C# type that crosses the Rust/il2cpp boundary — as a hook parameter, return value, method argument, or field — implements `Type`, the trait that tells the library which C# class a Rust type represents (`NAMESPACE`, `CLASS_NAME`) and how to look up its `Il2CppClass`/`Il2CppType`. Built-ins (`Il2CppObject`, `Il2CppString`, the numeric primitives, `Gc<T>`, ...) already implement it; you only need to reach for it yourself when wrapping a C# type that isn't provided out of the box.

C# types come in two shapes, and `Type` alone doesn't say which one a given type is:

- **Reference types** (classes, boxed values, strings) live on the GC heap and are always passed around by pointer — see [Memory management](#memory-management). Wrap them with `unsafe_impl_reference_type!`, which provides the `RefType` and `ObjectExt` extensions for free.
- **Value types** (structs, enums) are passed by raw bytes/copy, matching C#'s value semantics — unless the C# signature declares the parameter `ref`/`out`/`in`, in which case that particular value is passed by pointer instead. Wrap them with `unsafe_impl_value_type!` and get the `ValueType` extension trait (`invoke`, `invoke_void`, `as_boxed`) for free.

Both macros implement `Type`, plus the lower-level `Argument`/`Parameter`/`Return`/`Returned` traits, for you. All that's required from the struct itself is `#[repr(C)]` with fields laid out to match the real C# type's memory layout.

```rust
use quest_hook::libil2cpp::{unsafe_impl_reference_type, unsafe_impl_value_type, Il2CppArray, Il2CppObject, Type};

// A C# struct (UnityEngine.Vector3) - passed by value.
#[derive(Debug)]
#[repr(C)]
pub struct Vector3 { x: f32, y: f32, z: f32 }
unsafe_impl_value_type!(in quest_hook::libil2cpp for Vector3 => UnityEngine.Vector3);

// A C# class (System.Collections.Generic.List<T>) - passed by pointer/Gc<T>.
// Its layout mirrors the real List<T> object: an Il2CppObject header
// followed by its fields, and the Rust generic `T` maps onto C#'s generic
// parameter.
#[repr(C)]
pub struct List<T: Type> {
    object: Il2CppObject,
    items: *mut Il2CppArray<T>,
    size: i32,
}
unsafe_impl_reference_type!(in quest_hook::libil2cpp for List<T>.object => System.Collections.Generic.List<T>);
```

Once a type implements `Type`, it can be used directly as a `#[hook]` parameter/return type (see [Example](#example)) or passed to `invoke`/`invoke_void`. At each call site the library checks it against the target method's real C# signature — via the `Argument`/`Parameter`/`Return`/`Returned` traits, implemented automatically by the macros above — and panics on a mismatch rather than silently misinterpreting the bytes.

See the [`custom_type`](./examples/custom_type.rs) example for both macros used together in a hook.

## Cargo features

| Feature | Default | Description |
|---------|---------|-------------|
| `il2cpp_v31` | ✓ | Target il2cpp from Unity 2021+ |
| `il2cpp_v29` | | Target il2cpp from Unity 2019–2020 |
| `il2cpp_v24` | | Target il2cpp from Unity 2018 |
| `unity2018` | | Unity 2018 legacy alias for `il2cpp_v24` |
| `util` | ✓ | `setup()` helper — logging + panic handler via `tracing` |
| `cache` | ✓ | Cache class/method lookups for faster repeated access |
| `inline_hook` | ✓ | Function hooking on Android via a vendored inline-hook backend |
| `flamingo` | | Function hooking on Android `AArch64` via the Flamingo native hooking library |
| `retour` | ✓ | Function hooking on Windows/Linux/macOS via the `retour` crate |
| `bindgen` | ✓ | Generate il2cpp bindings at build time via `bindgen` |
| `serde` | | `Serialize`/`Deserialize` for il2cpp types |
| `trace` | | `tracing` instrumentation inside the library internals |

## Workspace crates

| Crate | Description |
|-------|-------------|
| [`libil2cpp`](./libil2cpp/) | Safe wrappers and raw bindings for Unity's libil2cpp |
| [`hook_backend`](./hook_backend/) | Cross-platform inline function hooking (`inline_hook` / `flamingo` / `retour` backends) |
| [`proc_macros`](./proc_macros/) | The `#[hook]` macro and other derive helpers |
| [`quest_build_helper`](./quest_build_helper/) | Build-script utilities for Quest mod projects |

## Contributing

Contributions are welcome, especially to the documentation and examples. Most of the discussions regarding the development of this library happen in the `#quest-mod-dev` channel of the [BSMG Discord server](https://discord.gg/beatsabermods).

Everything that can reasonably be done in Rust should be done in Rust. This library is, by nature, extremely unsafe and contains a lot of unsafe code — the goal is a Rust-friendly API surface, not the elimination of unsafety.

A decent understanding of both Rust and C++ is required for most contributions. The main reference is the libil2cpp source. Another useful reference is [beatsaber-hook](https://github.com/QuestPackageManager/beatsaber-hook).

## License

`quest_hook` is licensed under the [MIT License](./LICENSE).

## Credits

This library wouldn't exist without the invaluable help, feedback, and previous work from [Sc2ad](https://github.com/sc2ad).
