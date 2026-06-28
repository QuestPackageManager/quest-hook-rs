# `quest_build_helper`

Build-script utilities for Quest mod projects that use `quest-hook-rs`.

Provides helpers for three common `build.rs` tasks:

- Compiling C++ files with the correct il2cpp / cordl / fmt include paths and defines (`cc` module).
- Setting up linker flags for `cdylib` targets and linking QPM-managed `.so` libraries (`linker` module).
- Restoring [QPM](https://github.com/QuestPackageManager/QPM.CLI) packages before the build (`qpm` module).

## Cargo.toml

```toml
[build-dependencies]
quest_build_helper = { git = "https://github.com/StackDoubleFlow/quest-hook-rs.git" }
```

## Usage

### Typical `build.rs`

```rust
use std::path::Path;
use quest_build_helper::{cc::QuestCpp, linker, qpm};

fn main() {
    // 1. Restore QPM dependencies (downloads libflamingo.so, cordl headers, etc.)
    let qpm_dir = qpm::restore(Path::new("."), false).expect("QPM restore failed");

    let extern_dir = qpm_dir.join("extern");
    let libs_dir = extern_dir.join("libs");
    let includes_dir = extern_dir.join("includes");

    // 2. Set up linker defaults and link QPM-managed .so files
    linker::setup_linker_defaults();
    linker::linker_flags(&libs_dir);

    // 3. Compile a C++ source file with Quest-standard flags
    cc::Build::new()
        .file("src/my_cpp_wrapper.cpp")
        .add_quest_defaults()
        .add_quest_defines()
        .add_il2cpp_includes(&includes_dir)
        .add_cordl_includes(&includes_dir)
        .add_fmt_includes(&includes_dir)
        .compile("my_cpp_wrapper");
}
```

## API reference

### `cc::QuestCpp` trait

Extension trait on [`cc::Build`](https://docs.rs/cc) that adds Quest-specific builder methods:

| Method | Description |
|--------|-------------|
| `add_il2cpp_includes(include_dir)` | Add libil2cpp header paths under `extern/includes/libil2cpp/...` |
| `add_fmt_includes(include_dir)` | Add `{fmt}` header path |
| `add_cordl_includes(include_dir)` | Add bs-cordl generated header path |
| `add_quest_defines()` | Define `QUEST`, `UNITY_2021`, `UNITY_2022`, `HAS_CODEGEN`, etc. |
| `add_quest_defaults()` | Enable PIC, C++20, exceptions, RTTI, and link `c++_static` |

All methods return `&mut Self` so they can be chained.

### `linker`

| Function | Description |
|----------|-------------|
| `setup_linker_defaults()` | Emit `cargo:rustc-link-arg` flags: `--no-undefined`, `--gc-sections`, `-z defs`, and `libstatic=c++` on Android |
| `linker_flags(lib_path)` | Scan `lib_path` for `.so` files and emit `cargo:rustc-link-lib` for each |

### `qpm`

| Function | Description |
|----------|-------------|
| `qpm_bin() -> PathBuf` | Returns the QPM binary path (`QPM_PATH` env var or `"qpm"`) |
| `restore(path, copy_to_out) -> Result<PathBuf>` | Run `qpm restore` in `path`. If `copy_to_out` is `true`, copies `qpm.json`/`qpm.shared.json` to `$OUT_DIR/qpm` first to avoid mutating the source tree. Returns the directory where packages were restored. |

`restore` emits a `cargo:rerun-if-changed` directive for `qpm.json` so Cargo re-runs the build script when dependencies change.

#### QPM binary

QPM must be installed before building a crate that calls `qpm::restore`. Install it from the [QPM.CLI releases page](https://github.com/QuestPackageManager/QPM.CLI/releases) and ensure it is on `PATH`, or set the `QPM_PATH` environment variable.
