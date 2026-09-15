/* ffi_raii.h — RAII wrappers for notesplusplus-core FFI handles.
 *
 * Prevents resource leaks by tying handle lifetime to C++ scope.
 */

#ifndef NOTESPLUSPLUS_FFI_RAII_H
#define NOTESPLUSPLUS_FFI_RAII_H

#include <memory>
#include <QString>
#include "notesplusplus_core.h"

/* Custom deleters for each handle type */
struct AppPathsDeleter {
    void operator()(AppPaths* p) const { notes_core_app_paths_free(p); }
};
struct DbConnDeleter {
    void operator()(void* p) const { notes_core_db_close(p); }
};
struct SearchEngineDeleter {
    void operator()(FfiSearchEngine* p) const { notes_core_search_free(p); }
};
struct AgentSessionDeleter {
    void operator()(FfiAgentSession* p) const { notes_core_agent_free(p); }
};
struct SttDownloadDeleter {
    void operator()(FfiSttDownload* p) const { notes_core_stt_download_free(p); }
};
struct HttpServerDeleter {
    void operator()(HttpServerHandle* p) const { notes_core_server_stop(p); }
};

struct FfiStringDeleter {
    void operator()(char* s) const { notes_core_free_string(s); }
};

/* Smart pointer types */
using AppPathsPtr    = std::unique_ptr<AppPaths, AppPathsDeleter>;
using DbConnPtr      = std::unique_ptr<void, DbConnDeleter>;
using SearchPtr      = std::unique_ptr<FfiSearchEngine, SearchEngineDeleter>;
using AgentPtr       = std::unique_ptr<FfiAgentSession, AgentSessionDeleter>;
using SttDownloadPtr = std::unique_ptr<FfiSttDownload, SttDownloadDeleter>;
using HttpServerPtr  = std::unique_ptr<HttpServerHandle, HttpServerDeleter>;
using FfiString      = std::unique_ptr<char, FfiStringDeleter>;

/* Helper: wrap a returned char* into an FfiString and convert to QString */
inline QString ffiStringToQString(char* raw) {
    if (!raw) return QString();
    FfiString guard(raw);
    return QString::fromUtf8(guard.get());
}

/* Helper: wrap a returned char* into an FfiString and convert to std::string */
inline std::string ffiStringToStd(char* raw) {
    if (!raw) return std::string();
    FfiString guard(raw);
    return std::string(guard.get());
}

/* Helper: convert QString to temporary UTF-8 data for FFI calls.
 * The returned pointer is valid as long as the QByteArray is alive. */
inline const char* qstrToFFI(const QString& s) {
    return s.toUtf8().constData();
}

#endif /* NOTESPLUSPLUS_FFI_RAII_H */
