# STT test fixtures

`jfk.wav` is the well-known whisper.cpp sample recording (an excerpt of John F.
Kennedy's 1961 inaugural address: "...ask not what your country can do for you...").
It is vendored unmodified from `samples/jfk.wav` in the pinned whisper.cpp release
(see `../../../third_party/whisper.cpp/README.md`) and is used only by the opt-in,
`#[ignore]`d real-model integration test in `../../src/stt/engine.rs`
(`real_tiny_model_transcribes_jfk_fixture`), which requires a real `ggml-*.bin`
Whisper model supplied via the `WHISPER_MODEL_PATH` environment variable and is not
part of the default `cargo test` run.
