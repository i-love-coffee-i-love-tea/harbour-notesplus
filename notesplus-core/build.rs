fn main() {
    #[cfg(feature = "whisper")]
    build_whisper_cpp();
}

/// Compiles the vendored whisper.cpp / ggml sources (CPU-only) together with our thin
/// `whisper_bridge` C ABI shim, and links them into `notesplus-core`.
///
/// The sources live under `third_party/whisper.cpp/` and were vendored from a pinned
/// upstream release (see `third_party/whisper.cpp/LICENSE`). We deliberately avoid cmake
/// and bindgen/libclang here: only the small, hand-written `whisper_bridge` C ABI is
/// exposed to Rust (see `src/stt/whisper_ffi.rs`), so no bindings need to be generated at
/// build time and no GPU backends (CUDA/Metal/Vulkan/SYCL) are compiled in.
#[cfg(feature = "whisper")]
fn build_whisper_cpp() {
    let dir = "third_party/whisper.cpp";

    println!("cargo:rerun-if-changed={}", dir);

    let mut c_build = cc::Build::new();
    c_build
        .include(dir)
        .file(format!("{}/ggml.c", dir))
        .file(format!("{}/ggml-alloc.c", dir))
        .file(format!("{}/ggml-backend.c", dir))
        .file(format!("{}/ggml-quants.c", dir))
        .flag_if_supported("-std=gnu11")
        .flag_if_supported("-w")
        .define("_GNU_SOURCE", None)
        .define("NDEBUG", None);
    c_build.compile("ggml");

    let mut cxx_build = cc::Build::new();
    cxx_build
        .cpp(true)
        .include(dir)
        .file(format!("{}/whisper.cpp", dir))
        .file(format!("{}/whisper_bridge.cpp", dir))
        .flag_if_supported("-std=c++17")
        .flag_if_supported("-w")
        .define("NDEBUG", None);
    cxx_build.compile("whisper");

    println!("cargo:rustc-link-lib=pthread");
    println!("cargo:rustc-link-lib=m");
}
