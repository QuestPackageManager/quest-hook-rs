use std::env;
use std::path::PathBuf;

fn main() {
    #[cfg(feature = "bindgen")]
    run_bindgen();
}

#[cfg(feature = "bindgen")]
fn run_bindgen() {
    use std::process::Command;

    use quest_build_helper::qpm;

    #[cfg(feature = "il2cpp_v29")]
    let version = "0.2.0";

    #[cfg(feature = "il2cpp_v31")]
    let version = "0.4.0";

    #[cfg(not(any(feature = "il2cpp_v29", feature = "il2cpp_v31")))]
    compile_error!("No il2cpp version feature enabled: enable one of 'il2cpp_v29' or 'il2cpp_v31'");

    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    println!("Manifest dir: {}", manifest.display());

    println!(
        "cargo:rustc-link-search={}/extern/includes/libil2cpp/il2cpp/libil2cpp",
        manifest.display()
    );

    // qpm dependency add libil2cpp --version {version}
    Command::new(qpm::qpm_bin())
        .args(["dependency", "add", "libil2cpp", "--version", version])
        .current_dir(&manifest)
        .status()
        .expect("Failed to add qpm dependency");

    qpm::restore(&manifest, false).expect("Failed to restore qpm dependencies");

    println!("cargo:rerun-if-changed=wrapper.h");
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg(format!("-I{}", manifest.join("extern/includes/libil2cpp/il2cpp/libil2cpp").display()))
        .clang_arg(format!("-I{}", manifest.join("extern/includes/libil2cpp/il2cpp/external/baselib/Include").display()))
        .clang_arg(format!("-I{}", manifest.join("extern/includes/libil2cpp/il2cpp/external/baselib/Platforms/Android/Include").display()))
        .clang_arg(format!("-I{}", manifest.join("extern/includes").display()))
        .clang_arg(format!("-I{}", manifest.display()))
        .clang_arg("-v")
        .wrap_unsafe_ops(true)
        .sort_semantically(true)
        .generate()
        .expect("Unable to generate bindings");

    if cfg!(feature = "bindgen") {
        let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
        bindings
            .write_to_file(out_path.join("bindings.rs"))
            .expect("Couldn't write bindings!");
    }

    if env::var("WRITE_BINDINGS_LOCALLY").is_err() {
        return;
    }

    // write bindings to local file for easier inspection
    let current_dir = env::current_dir().expect("Failed to get current directory");
    let local_output_path = current_dir.join("bindings_out.rs");
    bindings
        .write_to_file(&local_output_path)
        .expect("Couldn't write bindings!");
    println!("Returning early");
    panic!("Written bindings to {}", local_output_path.display());
}
