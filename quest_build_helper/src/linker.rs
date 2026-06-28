use std::{collections::HashSet, fs, path::Path};

/// Setup default linker flags for cdylib building
pub fn setup_linker_defaults() {
    println!("cargo:rustc-link-arg=-Wl,--no-undefined");
    println!("cargo:rustc-link-arg=-Wl,--no-undefined-version");
    println!("cargo:rustc-link-arg=-Wl,--fatal-warnings");
    println!("cargo:rustc-link-arg=-Wl,--gc-sections");
    println!("cargo:rustc-link-arg=-Wl,-z,defs");

    // TODO: How to avoid this?
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "android" {
        println!("cargo:rustc-link-lib=static=c++");

        // println!("cargo:rustc-link-lib=static=c++abi");
        // println!("cargo:rustc-link-lib=static=unwind");
    }
}

/// Linker flags for dynamic libs in lib_path e.g qpm extern libs
/// This is needed to link dynamic libs when building cdylib
pub fn linker_flags(lib_path: &Path) {
    println!("cargo:rustc-link-search={}", lib_path.display());

    let mut to_link_libs = HashSet::new();

    // link dynamic libs
    for lib in fs::read_dir(lib_path).expect("Extern lib path not found") {
        let lib = lib.expect("Failed to read extern lib path").path();
        let Some(ext) = lib.extension() else { continue };
        if ext != "so" {
            continue;
        }
        let Some(filename) = lib.file_name() else {
            continue;
        };
        let Some(filename_str) = filename.to_str() else {
            continue;
        };

        if filename_str.starts_with("lib") && filename_str.ends_with(".so") {
            let lib_name = &filename_str[3..filename_str.len() - 3];
            to_link_libs.insert(lib_name.to_owned());
        }
    }

    for lib in &to_link_libs {
        if lib.ends_with(".debug.so")
            && to_link_libs.contains(lib.replace(".debug.so", ".so").as_str())
        {
            // skip debug lib if normal lib exists
            continue;
        }

        println!("cargo:rustc-link-lib={}", lib);
    }
}
