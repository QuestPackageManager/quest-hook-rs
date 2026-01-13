use std::{env, path::PathBuf};

fn main() {
    let workspace = env::var("CARGO_MANIFEST_DIR").unwrap();

    // Tell cargo to look for shared libraries in the specified directory
    println!("cargo:rustc-link-search={workspace}/extern/libs");

    // Tell cargo to tell rustc to link the system bzip2
    // shared library.
    println!("cargo:rustc-link-lib=flamingo");

    println!("cargo:rerun-if-changed=wrapper.h");
    println!("cargo:rerun-if-changed=extern/includes");

    // run qpm restore to ensure flamingo is available
    use std::process::Command;
    Command::new("qpm")
        .args(["restore"])
        .status()
        .expect("Failed to run qpm restore");

    // The bindgen::Builder is the main entry point
    // to bindgen, and lets you build up options for
    // the resulting bindings.
    let bindings = bindgen::Builder::default()
        // The input header we would like to generate
        // bindings for.
        .header("wrapper.h")
        .clang_arg("-I./extern/includes")
        // Tell cargo to invalidate the built crate whenever any of the
        // included header files changed.
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Finish the builder and generate the bindings.
        .generate()
        // Unwrap the Result and panic on failure.
        .expect("Unable to generate bindings");

    // Write the bindings to the $OUT_DIR/bindings.rs file.
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    // let target = std::env::var("TARGET").unwrap();
    // if target == "aarch64-linux-android" {
    //     cc::Build::new()
    //         .file("beatsaber-hook/shared/inline-hook/And64InlineHook.cpp")
    //         .compile("inline_hook");
    // }
}
