use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Tell Cargo to rerun this script if any of these files change
    println!("cargo:rerun-if-changed=resources/Info.plist");
    println!("cargo:rerun-if-changed=resources/entitlements.plist");

    // Copy resources to the output directory for app bundle creation
    let out_dir = env::var("OUT_DIR").unwrap();
    let resources_dir = Path::new("resources");

    if resources_dir.exists() {
        let target_dir = Path::new(&out_dir).join("resources");
        fs::create_dir_all(&target_dir).unwrap();

        for entry in fs::read_dir(resources_dir).unwrap() {
            let entry = entry.unwrap();
            let src = entry.path();
            let dst = target_dir.join(entry.file_name());
            fs::copy(&src, &dst).unwrap();
        }
    }

    // Set macOS deployment target
    println!("cargo:rustc-env=MACOSX_DEPLOYMENT_TARGET=12.0");
}
