// mod build_chromaprint;

fn main() {
    // build_chromaprint::build();

    // println!("cargo:rustc-link-search=native=./resources");

    // Link against chromaprint statically
    // println!("cargo:rustc-link-lib=static=chromaprint");

    // Link against C++ standard library (needed for static libraries)
    // #[cfg(target_os = "macos")]
    // println!("cargo:rustc-link-lib=dylib=c++");

    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-lib=dylib=msvcrt");
        println!("cargo:rustc-link-lib=dylib=vcruntime");
    }

    // Link against Accelerate framework (for vDSP functions)
    // println!("cargo:rustc-link-lib=framework=Accelerate");

    // println!("cargo:rerun-if-changed=resources/libchromaprint.a");
    tauri_build::build()
}
