use std::{
    env,
    path::{Path, PathBuf},
};

/// Returns the path to the QPM binary.
pub fn qpm_bin() -> PathBuf {
    PathBuf::from(env::var("QPM_PATH").unwrap_or_else(|_| "qpm".into()))
}

/// Restores QPM packages in the given path.
/// If `copy_to_out` is true, it will first copy the qpm.json and qpm.shared.json
/// from the manifest directory to the OUT_DIR, and then restore from there.
/// 
/// Errors if the qpm.json file is not found or if the QPM command fails.
/// 
/// Returns the path where the packages were restored. 
pub fn restore(path: &Path, copy_to_out: bool) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if copy_to_out {
        let out_path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("qpm");
        std::fs::create_dir_all(&out_path)
            .map_err(|e| format!("Failed to create out qpm directory: {}", e))?;

        // copy from manifest directory to out_path
        {
            let manifest_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
            // cp qpm.json to out_path
            let qpm_json_path = manifest_path.join("qpm.json");
            let qpm_json_out_path = out_path.join("qpm.json");
            if qpm_json_path.exists() {
                std::fs::copy(&qpm_json_path, &qpm_json_out_path)
                    .map_err(|e| format!("Failed to copy qpm.json: {}", e))?;
            }

            // cp qpm.shared.json to out_path
            let qpm_shared_json_path = manifest_path.join("qpm.shared.json");
            let qpm_shared_json_out_path = out_path.join("qpm.shared.json");
            if qpm_shared_json_path.exists() {
                std::fs::copy(&qpm_shared_json_path, &qpm_shared_json_out_path)
                    .map_err(|e| format!("Failed to copy qpm.shared.json: {}", e))?;
            }
        }

        restore(&out_path, false)?;
        return Ok(out_path);
    }

    let qpm_bin = qpm_bin();

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
    println!("cargo:rerun-if-changed={}", path.join("qpm.json").display());
    // println!(
    //     "cargo:rerun-if-changed={}",
    //     manifest_path.join("qpm.shared.json").display()
    // );

    Ok(path.to_path_buf())
}
