use std::env;
use std::path::PathBuf;

fn main() {
    #[cfg(feature = "bindgen")]
    run_bindgen();
}

#[cfg(feature = "bindgen")]
fn run_bindgen() {
    use std::process::Command;

    #[cfg(feature = "il2cpp_v29")]
    let version = "0.2.0";

    #[cfg(feature = "il2cpp_v31")]
    let version = "0.4.0";

    // qpm dependency add libil2cpp --version {version}
    // qpm restore
    Command::new("qpm")
        .args(["dependency", "add", "libil2cpp", "--version", version])
        .status()
        .expect("Failed to add qpm dependency");

    println!("cargo:rerun-if-changed=wrapper.h");
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg("-I./extern/includes/libil2cpp/il2cpp/libil2cpp")
        .clang_arg("-v")
        .wrap_unsafe_ops(true)
        .sort_semantically(true) // Incluye las cabeceras si es necesario
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
