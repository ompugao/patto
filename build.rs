use std::process::Command;

fn main() {
    let frontend_dir = "patto-preview-ui/";
    let watch_dir = "patto-preview-ui/src/";

    #[cfg(windows)]
    const NPM: &str = "npm.cmd"; // Or "npm.ps1" if you're explicitly using PowerShell
    #[cfg(not(windows))]
    const NPM: &str = "npm";

    // Only rerun build script if something in the frontend changes
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", watch_dir);

    // The preview server is the only consumer of the bundled web UI. Skipping npm
    // here is what lets the crate cross-compile (Android/iOS/wasm) as a library.
    if std::env::var_os("CARGO_FEATURE_PREVIEW").is_none() {
        return;
    }

    // Run `npm install`
    let status = Command::new(NPM)
        .arg("install")
        .current_dir(frontend_dir)
        .status()
        .expect("Failed to run npm install");
    if !status.success() {
        panic!("npm install failed");
    }

    // Run `npm run build`
    let status = Command::new(NPM)
        .args(["run", "build"])
        .current_dir(frontend_dir)
        .status()
        .expect("Failed to run npm run build");
    if !status.success() {
        panic!("npm run build failed");
    }
}
