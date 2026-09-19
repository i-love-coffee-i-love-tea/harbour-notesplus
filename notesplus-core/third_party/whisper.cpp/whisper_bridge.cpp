#include "whisper_bridge.h"
#include "whisper.h"

#include <cstdlib>
#include <cstring>
#include <string>

struct whisper_bridge_ctx {
    struct whisper_context *ctx;
};

extern "C" {

whisper_bridge_ctx *whisper_bridge_init(const char *model_path) {
    if (model_path == nullptr) {
        return nullptr;
    }

    // whisper.cpp's model loader does not fully validate malformed/truncated GGML
    // files before use (e.g. it may `vector::resize()` a garbage length read past
    // EOF), which can throw `std::length_error`/`std::bad_alloc` instead of failing
    // gracefully. Such C++ exceptions must never cross this `extern "C"` boundary
    // into Rust (doing so aborts the whole process), so we convert them into a plain
    // NULL return here.
    try {
        struct whisper_context_params cparams = whisper_context_default_params();
        cparams.use_gpu = false;

        struct whisper_context *wctx = whisper_init_from_file_with_params(model_path, cparams);
        if (wctx == nullptr) {
            return nullptr;
        }

        whisper_bridge_ctx *bridge = static_cast<whisper_bridge_ctx *>(malloc(sizeof(whisper_bridge_ctx)));
        if (bridge == nullptr) {
            whisper_free(wctx);
            return nullptr;
        }

        bridge->ctx = wctx;
        return bridge;
    } catch (...) {
        return nullptr;
    }
}

void whisper_bridge_free(whisper_bridge_ctx *bridge) {
    if (bridge == nullptr) {
        return;
    }
    if (bridge->ctx != nullptr) {
        whisper_free(bridge->ctx);
    }
    free(bridge);
}

int whisper_bridge_is_multilingual(whisper_bridge_ctx *bridge) {
    if (bridge == nullptr || bridge->ctx == nullptr) {
        return 0;
    }
    return whisper_is_multilingual(bridge->ctx);
}

char *whisper_bridge_transcribe(whisper_bridge_ctx *bridge, const float *samples, int n_samples,
                                 const char *language, int n_threads) {
    if (bridge == nullptr || bridge->ctx == nullptr || samples == nullptr || n_samples <= 0) {
        return nullptr;
    }

    // See the comment in `whisper_bridge_init`: never let a C++ exception raised inside
    // whisper.cpp escape across this `extern "C"` boundary into Rust.
    try {
        struct whisper_full_params wparams = whisper_full_default_params(WHISPER_SAMPLING_GREEDY);
        wparams.print_progress = false;
        wparams.print_special = false;
        wparams.print_realtime = false;
        wparams.print_timestamps = false;
        wparams.translate = false;
        wparams.no_context = true;
        wparams.no_timestamps = false;
        wparams.single_segment = false;
        wparams.no_speech_thold = 0.6f;
        wparams.logprob_thold = -1.0f;
        wparams.temperature = 0.0f;
        wparams.temperature_inc = 0.2f;
        wparams.n_threads = n_threads > 0 ? n_threads : 4;

        const bool is_multilingual = whisper_is_multilingual(bridge->ctx) != 0;
        const bool have_language = language != nullptr && language[0] != '\0' && strcmp(language, "auto") != 0;

        if (!is_multilingual) {
            wparams.language = "en";
            wparams.detect_language = false;
        } else if (have_language) {
            wparams.language = language;
            wparams.detect_language = false;
        } else {
            wparams.language = "auto";
            wparams.detect_language = false;
        }

        if (whisper_full(bridge->ctx, wparams, samples, n_samples) != 0) {
            return nullptr;
        }

        std::string result;
        const int n_segments = whisper_full_n_segments(bridge->ctx);
        for (int i = 0; i < n_segments; ++i) {
            const char *text = whisper_full_get_segment_text(bridge->ctx, i);
            if (text != nullptr) {
                result += text;
            }
        }

        if (result.empty() && n_segments > 0) {
            for (int i = 0; i < n_segments; ++i) {
                const int n_tokens = whisper_full_n_tokens(bridge->ctx, i);
                for (int j = 0; j < n_tokens; ++j) {
                    const char *tok = whisper_full_get_token_text(bridge->ctx, i, j);
                    if (tok != nullptr && tok[0] != '[' && tok[0] != '<') {
                        result += tok;
                    }
                }
            }
        }

        const size_t start = result.find_first_not_of(" \t\n\r");
        const size_t end = result.find_last_not_of(" \t\n\r");
        const std::string trimmed =
            (start == std::string::npos) ? std::string() : result.substr(start, end - start + 1);

        char *out = static_cast<char *>(malloc(trimmed.size() + 1));
        if (out == nullptr) {
            return nullptr;
        }
        memcpy(out, trimmed.c_str(), trimmed.size() + 1);
        return out;
    } catch (...) {
        return nullptr;
    }
}

void whisper_bridge_free_string(char *s) {
    if (s != nullptr) {
        free(s);
    }
}

} // extern "C"
