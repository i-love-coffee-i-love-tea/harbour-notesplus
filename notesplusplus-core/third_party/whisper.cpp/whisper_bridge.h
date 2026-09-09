#ifndef NOTESPLUSPLUS_WHISPER_BRIDGE_H
#define NOTESPLUSPLUS_WHISPER_BRIDGE_H

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque handle wrapping a loaded whisper.cpp `whisper_context`. */
typedef struct whisper_bridge_ctx whisper_bridge_ctx;

/* Loads a GGML Whisper model from `model_path`. Returns NULL on failure. */
whisper_bridge_ctx *whisper_bridge_init(const char *model_path);

/* Frees a context created by `whisper_bridge_init`. Safe to call with NULL. */
void whisper_bridge_free(whisper_bridge_ctx *ctx);

/* Returns non-zero if the loaded model supports languages other than English. */
int whisper_bridge_is_multilingual(whisper_bridge_ctx *ctx);

/*
 * Transcribes 16kHz mono `f32` PCM `samples` (length `n_samples`) using the loaded model.
 * `language` may be NULL/empty/"auto" for automatic language detection on multilingual models.
 * Returns a heap-allocated, NUL-terminated UTF-8 string on success (must be freed with
 * `whisper_bridge_free_string`), or NULL on failure.
 */
char *whisper_bridge_transcribe(whisper_bridge_ctx *ctx, const float *samples, int n_samples,
                                 const char *language, int n_threads);

/* Frees a string returned by `whisper_bridge_transcribe`. Safe to call with NULL. */
void whisper_bridge_free_string(char *s);

#ifdef __cplusplus
}
#endif

#endif /* NOTESPLUSPLUS_WHISPER_BRIDGE_H */
