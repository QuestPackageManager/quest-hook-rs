# `libil2cpp`

Safe Rust wrappers and raw bindings for Unity's [libil2cpp](https://github.com/Unity-Technologies/il2cpp) C++ runtime.

This crate is the core of `quest_hook`. It provides typed access to il2cpp objects, classes, methods, strings, and arrays directly from Rust, as well as raw FFI types for lower-level work.

## Cargo.toml

```toml
[dependencies]
libil2cpp = { git = "https://github.com/StackDoubleFlow/quest-hook-rs.git" }
```

You **must** enable exactly one il2cpp version feature:

```toml
[dependencies]
libil2cpp = { git = "https://github.com/StackDoubleFlow/quest-hook-rs.git", features = ["il2cpp_v31"] }
```

## Features

| Feature | Description |
|---------|-------------|
| `il2cpp_v31` | il2cpp from Unity 2021+ |
| `il2cpp_v29` | il2cpp from Unity 2019–2020 |
| `il2cpp_v24` | il2cpp from Unity 2018 |
| `unity2018` | Unity 2018 (legacy alias for `il2cpp_v24`) |
| `bindgen` | Generate bindings at build time via `bindgen` (recommended) |
| `cache` | Cache class and method lookups for faster repeated access |
| `serde` | `Serialize`/`Deserialize` for il2cpp types |
| `trace` | `tracing` instrumentation inside library internals |

## Key types

| Type | Description |
|------|-------------|
| `Il2CppObject` | Base type for all managed (reference) il2cpp objects |
| `Il2CppClass` | Represents a managed type; used for method/field lookup |
| `Il2CppString` | Managed UTF-16 string |
| `Il2CppArray<T>` | Managed array |
| `Il2CppException` | Managed exception returned from method invocations |
| `MethodInfo` | Metadata for a single managed method |
| `FieldInfo` | Metadata for a single managed field |
| `Gc<T>` | Garbage-collected pointer wrapper |
| `ByRef<T>` / `ByRefMut<T>` | Pass managed values by reference across the FFI boundary |

## Usage

### Invoking methods on objects

```rust
use libil2cpp::{Il2CppObject, Il2CppString};

fn log_scene_name(scene: &mut Il2CppObject) {
    let name: &Il2CppString = scene.invoke("get_name", ()).unwrap();
    println!("Scene: {}", name.as_str().unwrap());
}
```

### Looking up a class and calling a static method

```rust
use libil2cpp::Il2CppClass;

let class = Il2CppClass::find("UnityEngine", "Debug").unwrap();
let method = class.find_method::<(&Il2CppString,), (), 1>("Log").unwrap();
// unsafe: you must ensure the method signature matches
unsafe { method.invoke_unchecked(std::ptr::null_mut(), (msg,)) }.unwrap();
```

### Implementing custom il2cpp types

Use the provided macros to teach `libil2cpp` about your Rust types so they can be used as method arguments and return values.

```rust
use libil2cpp::unsafe_impl_value_type;

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

// Maps this Rust struct to the UnityEngine.Vector3 managed value type
unsafe_impl_value_type!(in libil2cpp for Vector3 => UnityEngine.Vector3);
```

```rust
use libil2cpp::{unsafe_impl_reference_type, Il2CppObject, Il2CppArray, Type};

#[repr(C)]
pub struct List<T: Type> {
    object: Il2CppObject,
    items: *mut Il2CppArray<T>,
    size: i32,
}

// Maps to System.Collections.Generic.List<T>
unsafe_impl_reference_type!(in libil2cpp for List<T> => System.Collections.Generic.List<T>);
```

## Module layout

```
libil2cpp/
├── src/
│   ├── array.rs          # Il2CppArray<T>
│   ├── byref.rs          # ByRef / ByRefMut wrappers
│   ├── class.rs          # Il2CppClass — type/method/field lookup
│   ├── exception.rs      # Il2CppException
│   ├── field_info.rs     # FieldInfo
│   ├── gc.rs             # Gc<T> garbage-collected pointer
│   ├── method_info.rs    # MethodInfo, Result, Void
│   ├── object.rs         # Il2CppObject
│   ├── string.rs         # Il2CppString
│   ├── ty.rs             # Il2CppType / Il2CppReflectionType
│   ├── valuetype.rs      # ValueType helpers
│   ├── typecheck/        # Argument/Parameter/Return type checking
│   └── raw/              # Raw C types and dynamically loaded functions
```
