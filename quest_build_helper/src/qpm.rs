use std::{env, path::{Path, PathBuf}};


pub fn restore(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let qpm_bin = PathBuf::from(env::var("QPM_PATH").unwrap_or_else(|_| "qpm".into()));

    if !path.join("qpm.json").exists() {
        return Err(format!("qpm.json not found in {}", path.display()).into());
    }

    let mut cmd = std::process::Command::new(qpm_bin);
    cmd.current_dir(path)
        .arg("restore")
        // .arg("--quiet")
        .status()
        .map_err(|e| format!("Failed to run qpm: {}", e))?;

    // change if qpm.shared.json modified
    println!(
        "cargo:rerun-if-changed={}",
        path.join("qpm.json").display()
    );
    // println!(
    //     "cargo:rerun-if-changed={}",
    //     manifest_path.join("qpm.shared.json").display()
    // );

    Ok(())
}