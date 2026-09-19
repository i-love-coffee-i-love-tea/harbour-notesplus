# Vendored whisper.cpp (CPU-only)

This directory contains a pinned, trimmed-down snapshot of
[ggml-org/whisper.cpp](https://github.com/ggml-org/whisper.cpp), used to provide real,
offline Whisper speech-to-text inference for `notesplus-core` (see
`../../src/stt/engine.rs` and `../../src/stt/whisper_ffi.rs`).

- **Pinned version:** `v1.6.2` (tag `v1.6.2`, released 2024-05-27).
- **License:** MIT (see `LICENSE` in this directory; unmodified from upstream).
- **Vendored files:** only the CPU-only core is kept:
  - `ggml.c` / `ggml.h`, `ggml-alloc.c` / `.h`, `ggml-backend.c` / `.h` (+
    `ggml-backend-impl.h`), `ggml-quants.c` / `.h`, `ggml-common.h`, `ggml-impl.h`
  - `whisper.cpp` / `whisper.h`
  - `whisper_bridge.cpp` / `.h` -- **not** from upstream; a small hand-written C ABI
    shim we added (see below).

  GPU backends (CUDA/Metal/Vulkan/SYCL/OpenCL/Kompute/RPC), CoreML, OpenVINO, and the
  llamafile SGEMM path were intentionally **not** vendored: they are all guarded by
  `#ifdef`s that are never defined by our `build.rs`, so leaving out the corresponding
  source files keeps the build CPU-only, dependency-free, and portable to Sailfish OS
  ARM32/ARM64 targets without cmake.

## Why a hand-written bridge instead of bindgen?

`whisper.h`'s `whisper_full_params` struct is large, versioned, and contains nested
anonymous structs plus several function-pointer callback types. Replicating its exact
`#[repr(C)]` layout by hand in Rust (or via a bindgen snapshot that must be kept in
sync with the pinned version) is fragile and easy to get subtly wrong.

Instead, `whisper_bridge.h`/`whisper_bridge.cpp` expose a tiny, hand-written C ABI
(`whisper_bridge_init`, `_free`, `_is_multilingual`, `_transcribe`, `_free_string`)
with only primitive types (pointers, ints, floats, C strings) in its signatures. All
whisper.cpp struct manipulation happens on the C++ side, which we compile ourselves
and fully control. This means:

- No `bindgen`/`libclang` dependency at build time (important for the Sailfish SDK,
  which does not ship these by default).
- No ABI-layout risk on the Rust side (see `../../src/stt/whisper_ffi.rs`).
- whisper.cpp's model loader does not fully validate malformed/truncated GGML input
  (it can throw C++ exceptions such as `std::length_error`/`std::bad_alloc` instead of
  failing gracefully); `whisper_bridge.cpp` wraps every entry point in `try { ... }
  catch (...) { return nullptr; }` so such exceptions never cross the `extern "C"`
  boundary into Rust (which would otherwise abort the whole process).

## Upgrading the pinned version

1. Download the new release tarball from
   `https://github.com/ggml-org/whisper.cpp/archive/refs/tags/<tag>.tar.gz`.
2. Replace the upstream files listed above (keep `whisper_bridge.*` and this README).
3. Re-run `cargo test -p notesplus-core stt::` and, if possible, the real-model
   `#[ignore]`d test in `../../src/stt/engine.rs` with `WHISPER_MODEL_PATH` set to a
   real `ggml-*.bin` model to confirm inference still works end-to-end.
