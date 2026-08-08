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

A decent understanding of both Rust and C++ is required for most contributions. The main reference is the libil2cpp source. Another useful reference is [beatsaber-hook](https://github.com/sc2ad/beatsaber-hook).

## License

`quest_hook` is licensed under the [MIT License](./LICENSE).

## Credits

This library wouldn't exist without the invaluable help, feedback, and previous work from [Sc2ad](https://github.com/sc2ad).
