use std::env;
use std::path::PathBuf;

fn main() {
    // Skip building on docs.rs
    if env::var("DOCS_RS").is_ok() {
        return;
    }

    // Inform cargo's cfg checker about our conditional `draco_ffi_disabled` cfg so
    // the `check-cfg` lint does not warn about it being unexpected.
    println!("cargo:rustc-check-cfg=cfg(draco_ffi_disabled)");

    // Path to the original Draco source
    let draco_src = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("src");

    // Path to build directory with pre-built libraries
    let draco_build = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("build-original");

    // Check if libraries exist
    let lib_path = draco_build.join("src/draco/Release");
    if !lib_path.exists() {
        println!(
            "cargo:warning=C++ Draco library not found at {:?}. FFI features will be disabled.",
            lib_path
        );
        println!("cargo:rustc-cfg=draco_ffi_disabled");
        return;
    }

    // Compile our FFI wrapper
    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file("cpp/draco_ffi.cpp")
        .include(&draco_src)
        // The build dir has draco_features.h at build-original/draco/draco_features.h
        // but headers include it as "draco/draco_features.h", so we include the parent
        .include(&draco_build)
        .flag_if_supported("/std:c++17")
        .flag_if_supported("-std=c++17")
        .opt_level(3);

    // Windows-specific settings
    if cfg!(target_os = "windows") {
        build.flag("/EHsc");
    }

    build.compile("draco_ffi_wrapper");
    // Ensure the compiled wrapper is linked into all test binaries
    println!("cargo:rustc-link-lib=static=draco_ffi_wrapper");

    // Link to the pre-built Draco library
    println!("cargo:rustc-link-search=native={}", lib_path.display());
    println!("cargo:rustc-link-lib=static=draco");

    // Link all the component libraries (Draco builds as multiple static libs)
    let component_libs = [
        "draco_animation",
        "draco_animation_dec",
        "draco_animation_enc",
        "draco_attributes",
        "draco_compression_attributes_dec",
        "draco_compression_attributes_enc",
        "draco_compression_attributes_pred_schemes_enc",
        "draco_compression_bit_coders",
        "draco_compression_decode",
        "draco_compression_encode",
        "draco_compression_entropy",
        "draco_compression_mesh_dec",
        "draco_compression_mesh_enc",
        "draco_compression_point_cloud_dec",
        "draco_compression_point_cloud_enc",
        "draco_core",
        "draco_mesh",
        "draco_metadata",
        "draco_metadata_dec",
        "draco_metadata_enc",
        "draco_points_dec",
        "draco_points_enc",
        "draco_point_cloud",
        "draco_src_io",
    ];

    for lib_name in component_libs {
        let lib_dir = draco_build.join(format!("src/draco/{}.dir/Release", lib_name));
        if lib_dir.exists() {
            println!("cargo:rustc-link-search=native={}", lib_dir.display());
            println!("cargo:rustc-link-lib=static={}", lib_name);
        }
    }

    // Link C++ standard library
    if cfg!(target_os = "windows") {
        // MSVC links automatically
    } else if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=c++");
    } else {
        println!("cargo:rustc-link-lib=stdc++");
    }

    println!("cargo:rerun-if-changed=cpp/draco_ffi.cpp");
}
