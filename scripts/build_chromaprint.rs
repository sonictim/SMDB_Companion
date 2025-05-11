use std::env;
use std::path::PathBuf;

pub fn build() {
    let root_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let chromaprint_path = root_path.join("vendor").join("chromaprint");

    println!(
        "cargo:warning=Building chromaprint from {}",
        chromaprint_path.display()
    );

    let dst = cmake::Config::new(&chromaprint_path)
        .define("CMAKE_POSITION_INDEPENDENT_CODE", "ON")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("BUILD_TOOLS", "OFF")
        .define("BUILD_TESTS", "OFF")
        .define("WITH_AVFFT", "OFF")
        .build();

    println!("cargo:rustc-link-search=native={}/lib", dst.display());
    println!("cargo:rustc-link-lib=static=chromaprint");

    println!("cargo:warning=Library directory: {}/lib", dst.display());
    println!("cargo:rerun-if-changed=vendor/chromaprint");

    let lib_path = dst.join("lib");
    let lib_file = lib_path.join("libchromaprint.a");

    if lib_file.exists() {
        println!(
            "cargo:warning=Library file exists at: {}",
            lib_file.display()
        );
    } else {
        println!(
            "cargo:warning=Library file NOT FOUND at: {}",
            lib_file.display()
        );

        // List directory contents to debug
        if let Ok(entries) = std::fs::read_dir(&lib_path) {
            println!("cargo:warning=Directory contents:");
            for entry in entries {
                if let Ok(entry) = entry {
                    println!("cargo:warning=  {}", entry.path().display());
                }
            }
        }
    }
}
