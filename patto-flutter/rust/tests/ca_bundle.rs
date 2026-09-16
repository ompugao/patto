//! The CA bundle has to load, otherwise every HTTPS fetch fails to verify.

#[test]
fn a_pem_bundle_loads() {
    let bundle = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../assets/cacert.pem")
        .canonicalize()
        .expect("assets/cacert.pem is missing");

    rust_lib_patto_flutter::api::git::git_init_runtime(bundle.to_string_lossy().to_string())
        .expect("the bundled CA file should load");
}
