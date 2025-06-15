use std::process::Command;

fn main() {
    let frontend_dir = "patto-preview-next/";

    // Only rerun build script if something in the frontend changes
    println!("cargo:rerun-if-changed={}", frontend_dir);

    // Run `npm install`
    let status = Command::new("npm")
        .arg("install")
        .current_dir(frontend_dir)
        .status()
        .expect("Failed to run npm install");
    if !status.success() {
        panic!("npm install failed");
    }

    // Run `npm run build`
    let status = Command::new("npm")
        .args(["run", "build"])
        .current_dir(frontend_dir)
        .status()
        .expect("Failed to run npm run build");
    if !status.success() {
        panic!("npm run build failed");
    }
}
