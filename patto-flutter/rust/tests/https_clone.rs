#[test]
#[ignore]
fn clone_over_https_with_the_bundled_roots() {
    let bundle = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../assets/cacert.pem")
        .canonicalize()
        .unwrap();
    rust_lib_patto_flutter::api::git::git_init_runtime(bundle.to_string_lossy().to_string())
        .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let result = rust_lib_patto_flutter::api::git::git_clone(
        "https://github.com/octocat/Hello-World.git".to_string(),
        dir.path().to_string_lossy().to_string(),
        None,
        rust_lib_patto_flutter::api::git::GitCreds {
            username: String::new(),
            token: String::new(),
        },
        |_| {},
    );
    result.expect("clone should succeed");
    assert!(dir.path().join("README").exists());
}
