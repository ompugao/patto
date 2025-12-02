fn main() {
    println!("cargo:rerun-if-changed=src/parser.c");
    println!("cargo:rerun-if-changed=src/scanner.c");
    println!("cargo:rerun-if-changed=src/grammar.json");
    println!("cargo:rerun-if-changed=src/node-types.json");

    cc::Build::new()
        .include("src")
        .file("src/parser.c")
        .file("src/scanner.c")
        .compile("tree-sitter-patto");
}
