use std::env;
use std::path::PathBuf;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // Generate C header file
    let header_path = PathBuf::from(&crate_dir).join("include").join("draco_core.h");

    // Ensure include directory exists
    std::fs::create_dir_all(header_path.parent().unwrap()).unwrap();

    let bindings = cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_include_guard("DRACO_CORE_H")
        .with_namespace("draco_core")
        .with_language(cbindgen::Language::C)
        .generate()
        .expect("Unable to generate bindings");

    let success = bindings.write_to_file(&header_path);
    if !success {
        panic!("Couldn't write bindings!");
    }

    // Write the header with additional C++ compatibility
    let mut content = String::new();
    content.push_str("// GENERATED FILE -- DO NOT EDIT\n\n");
    content.push_str("#ifndef DRACO_CORE_H\n");
    content.push_str("#define DRACO_CORE_H\n\n");
    content.push_str("#include <stddef.h>\n");
    content.push_str("#include <stdint.h>\n");
    content.push_str("#include <stdbool.h>\n\n");

    // Read the generated content
    let generated_content = std::fs::read_to_string(&header_path)
        .expect("Couldn't read generated header");

    content.push_str(&generated_content);

    content.push_str("\n#ifdef __cplusplus\n");
    content.push_str("}\n");
    content.push_str("#endif\n\n");
    content.push_str("#endif // DRACO_CORE_H\n");

    std::fs::write(&header_path, content)
        .expect("Couldn't write header file");

    println!("cargo:warning=C header generated at: {}", header_path.display());

    // Tell cargo to rerun this script if the source files change
    println!("cargo:rerun-if-changed=src/");
    println!("cargo:rerun-if-changed=cbindgen.toml");

    // Emit the include directory for other crates
    let include_dir = PathBuf::from(&crate_dir).join("include");
    println!("cargo:include={}", include_dir.display());
}