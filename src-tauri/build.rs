fn main() {
    println!("cargo:rustc-link-search=native=./resources");

    // Link against chromaprint statically
    println!("cargo:rustc-link-lib=static=chromaprint");

    // Link against C++ standard library (needed for static libraries)
    println!("cargo:rustc-link-lib=dylib=c++");

    // Link against Accelerate framework (for vDSP functions)
    println!("cargo:rustc-link-lib=framework=Accelerate");

    println!("cargo:rerun-if-changed=resources/libchromaprint.a");
    tauri_build::build()
}
