# `quest_hook_proc_macros`

Procedural macros for `quest_hook`. This crate is re-exported through `quest_hook` and `libil2cpp` — you do not normally depend on it directly.

## Cargo.toml

Use `quest_hook` instead:

```toml
[dependencies]
quest_hook = { git = "https://github.com/StackDoubleFlow/quest-hook-rs.git" }
```

If you only need the type-mapping macros from `libil2cpp`:

```toml
[dependencies]
libil2cpp = { git = "https://github.com/StackDoubleFlow/quest-hook-rs.git", features = ["il2cpp_v31"] }
```

## Macros

### `#[hook("namespace", "ClassName", "MethodName")]`

The primary user-facing macro. Transforms a plain Rust function into a complete hook definition: a static `Hook` instance, a `Hook` trait implementation, and boilerplate for calling the original function.

```rust
use quest_hook::hook;
use quest_hook::libil2cpp::{Il2CppObject, Il2CppString};

#[hook("UnityEngine.SceneManagement", "SceneManager", "SetActiveScene")]
fn set_active_scene(scene: &mut Il2CppObject) -> bool {
    let name: &Il2CppString = scene.invoke("get_name", ()).unwrap();
    println!("Scene: {}", name);

    // Call through to the original method
    set_active_scene.original(scene)
}

// In your load() function:
set_active_scene.install().unwrap();
```

The macro accepts exactly three string-literal arguments: namespace, class name, and method name. It generates:

- A hidden inner function with the actual hook body.
- A unit struct named after the function (in PascalCase).
- A `static` instance of that struct with the same name as the function.
- An impl of the `quest_hook::Hook` trait on the struct.

### `unsafe_impl_value_type!`

Registers a Rust `#[repr(C)]` struct as the Rust representation of an il2cpp **value type** (struct). Required for the type to be usable as a method argument or return value.

```rust
use libil2cpp::unsafe_impl_value_type;

#[repr(C)]
pub struct Vector3 { pub x: f32, pub y: f32, pub z: f32 }

unsafe_impl_value_type!(in libil2cpp for Vector3 => UnityEngine.Vector3);
```

### `unsafe_impl_reference_type!`

Same as `unsafe_impl_value_type!` but for il2cpp **reference types** (classes). The Rust struct must begin with an `Il2CppObject` field.

```rust
use libil2cpp::{unsafe_impl_reference_type, Il2CppObject, Il2CppArray, Type};

#[repr(C)]
pub struct List<T: Type> {
    object: Il2CppObject,
    items: *mut Il2CppArray<T>,
    size: i32,
}

unsafe_impl_reference_type!(in libil2cpp for List<T> => System.Collections.Generic.List<T>);
```

### Internal macros

These are used by `libil2cpp` and `quest_hook` internally and are not part of the public API:

| Macro | Purpose |
|-------|---------|
| `identity` | No-op attribute used as a stand-in for `#[instrument]` when the `trace` feature is disabled |
| `impl_arguments_parameters` | Generates `Arguments`/`Parameters` impls for tuples up to a fixed arity |
| `impl_generics` | Generates `Generics` impls for generic il2cpp type tuples |
| `il2cpp_functions` | Generates the dynamic function loader for raw il2cpp functions |
