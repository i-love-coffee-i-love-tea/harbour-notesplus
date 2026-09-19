/* BridgeContext.h — Shared context for NotesBridge domain classes.
 *
 * Plain struct (not QObject) providing common dependencies.
 * Constructed once by NotesBridge, passed by const& to each domain class.
 */

#ifndef BRIDGECONTEXT_H
#define BRIDGECONTEXT_H

#include <QString>
#include <atomic>
#include <functional>
#include <memory>

struct BridgeContext {
    std::function<void*()>              rawConn;           // returns current DB connection
    QString                             notesPath;         // absolute notes directory
    QString                             dataDir;           // absolute data directory
    std::shared_ptr<std::atomic<bool>>  alive;             // lifetime guard
    std::function<bool()>               dropComments;      // current dropComments value
    std::function<QString()>            themeColorsJson;   // current theme colors JSON
    std::function<void(const QString&)> reportError;       // bound to NotesBridge::reportError
    std::function<QString()>            buildOptionsJson;  // bound to NotesBridge::buildOptionsJson
};

#endif /* BRIDGECONTEXT_H */
