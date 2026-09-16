/* SearchManager.cpp — FTS search, background search execution, preview
 * generation, and linkable-pages lookup for Notes++.
 *
 * Implementation of the SearchManager class.  Each search creates its own
 * FFI engine via notes_core_search_new() / notes_core_search_start(),
 * runs it on a background thread (QtConcurrent::run), and delivers results
 * back to the Qt main thread via QMetaObject::invokeMethod on the caller-
 * supplied signalTarget.
 */

#include "SearchManager.h"

#include <QDir>
#include <QFile>
#include "../ffi/ffi_raii.h"

/* ================================================================== */
/*  Static helpers                                                     */
/* ================================================================== */

/// Build the db_path as a QString (same logic as NotesBridge.cpp).
QString SearchManager::dbPathFor(const QString &dataDir)
{
    return dataDir + QLatin1Char('/') +
           ffiStringToQString(notes_core_const_db_filename());
}

/* ================================================================== */
/*  Constructor                                                        */
/* ================================================================== */

SearchManager::SearchManager(const BridgeContext &ctx)
    : m_ctx(ctx)
{
}

/* ================================================================== */
/*  search() — clear state (synchronous)                               */
/* ================================================================== */

void SearchManager::search(const QString &query)
{
    m_searchQuery   = query;
    m_searchResults.clear();
    m_searchLoading = false;
}

/* ================================================================== */
/*  do_search() — start async FTS search                               */
/* ================================================================== */

void SearchManager::do_search(const QString &query, QObject *signalTarget)
{
    m_searchQuery = query;

    if (query.trimmed().isEmpty()) {
        m_searchResults.clear();
        m_searchLoading = false;
        return;
    }

    /* Clear any previous pending result */
    {
        std::lock_guard<std::mutex> lk(m_pendingSearchMutex);
        m_pendingSearchHits.reset();
        m_pendingSearchError.clear();
    }
    {
        std::lock_guard<std::mutex> lk(m_pendingPreviewMutex);
        m_pendingPreviews.reset();
    }

    m_searchLoading = true;

    const int gen = ++m_searchGeneration;
    const std::string dbP = dbPathFor(m_ctx.dataDir).toStdString();
    const std::string q   = query.toStdString();

    auto alive = m_ctx.alive;
    auto &ctx  = m_ctx;

    QtConcurrent::run([this, alive, dbP, q, gen, signalTarget, &ctx]() {
        /* Create a fresh search engine for this query */
        FfiSearchEngine *engine = notes_core_search_new();
        int rc = notes_core_search_start(engine, dbP.c_str(), q.c_str());

        if (rc < 0) {
            notes_core_search_free(engine);
            if (gen != m_searchGeneration.load()) return;
            if (!alive->load(std::memory_order_acquire)) return;
            std::lock_guard<std::mutex> lk(m_pendingSearchMutex);
            m_pendingSearchError = QStringLiteral("Search failed to start");
            m_pendingSearchHits  = QList<SearchHit>();
            return;
        }

        /* Poll FFI engine until complete */
        char *outJson = nullptr;
        for (int i = 0; i < 600; ++i) {       /* ~30 s max */
            rc = notes_core_search_poll(engine, &outJson);
            if (rc != 0) break;                /* 1=ready, -1=error */
            std::this_thread::sleep_for(std::chrono::milliseconds(50));
        }

        if (gen != m_searchGeneration.load()) {
            /* A newer search superseded this one */
            if (outJson) notes_core_free_string(outJson);
            notes_core_search_free(engine);
            return;
        }

        if (rc == 1 && outJson) {
            /* Parse the JSON results */
            QString resultsStr = ffiStringToQString(outJson);
            QJsonDocument doc = QJsonDocument::fromJson(resultsStr.toUtf8());

            QList<SearchHit> hits;
            if (doc.isArray()) {
                for (const QJsonValue &v : doc.array()) {
                    QJsonObject o = v.toObject();
                    SearchHit h;
                    h.title      = o[QStringLiteral("title")].toString();
                    h.filename   = o[QStringLiteral("filename")].toString();
                    h.groupPath  = o[QStringLiteral("group_path")].toString();
                    h.fullPath   = o[QStringLiteral("full_path")].toString();
                    h.snippet    = o[QStringLiteral("snippet")].toString();
                    h.createdAt  = o[QStringLiteral("created_at")].toString();
                    h.updatedAt  = o[QStringLiteral("updated_at")].toString();
                    h.blockCount = o[QStringLiteral("block_count")].toInt();
                    h.previewJson = QStringLiteral("[]");
                    hits.append(h);
                }
            }

            if (!alive->load(std::memory_order_acquire)) return;

            {
                std::lock_guard<std::mutex> lk(m_pendingSearchMutex);
                m_pendingSearchHits = std::move(hits);
            }

            QMetaObject::invokeMethod(signalTarget, "poll_search", Qt::QueuedConnection);

            /* Kick off background preview generation */
            auto alive2 = ctx.alive;
            auto notesPath = ctx.notesPath;
            auto dropCommentsFn = ctx.dropComments;

            QtConcurrent::run([this, alive2, gen, signalTarget,
                               notesPath, dropCommentsFn]() {
                if (gen != m_searchGeneration.load()) return;
                if (!alive2->load(std::memory_order_acquire)) return;

                QList<SearchHit> snapshot;
                {
                    std::lock_guard<std::mutex> lk(m_pendingSearchMutex);
                    if (!m_pendingSearchHits.has_value()) return;
                    snapshot = *m_pendingSearchHits;
                }

                const std::string nd = notesPath.toStdString();
                const bool drop = dropCommentsFn();

                QMap<QString, QString> previewMap;
                for (const SearchHit &h : snapshot) {
                    if (gen != m_searchGeneration.load()) return;
                    const std::string fp = h.fullPath.toStdString();
                    char *src = notes_core_page_get_source(nd.c_str(), fp.c_str());
                    QString content = ffiStringToQString(src);
                    if (content.isEmpty()) continue;

                    char *blocks = notes_core_parse_blocks_json(
                        content.toUtf8().constData(), drop ? 1 : 0);
                    previewMap[h.fullPath] = ffiStringToQString(blocks);
                }

                if (gen != m_searchGeneration.load()) return;
                {
                    std::lock_guard<std::mutex> lk(m_pendingPreviewMutex);
                    m_pendingPreviews = std::move(previewMap);
                }

                QMetaObject::invokeMethod(signalTarget, "poll_search_previews",
                                          Qt::QueuedConnection);
            }).detach();

        } else {
            if (!alive->load(std::memory_order_acquire)) return;
            std::lock_guard<std::mutex> lk(m_pendingSearchMutex);
            m_pendingSearchError = QStringLiteral("Search failed");
            m_pendingSearchHits  = QList<SearchHit>();

            QMetaObject::invokeMethod(signalTarget, "poll_search", Qt::QueuedConnection);
        }

        notes_core_search_free(engine);
    }).detach();
}

/* ================================================================== */
/*  poll_search() — collect background search results                  */
/* ================================================================== */

SearchPollResult SearchManager::poll_search()
{
    SearchPollResult result;

    QList<SearchHit> hits;
    QString error;
    bool hasResult = false;

    {
        std::lock_guard<std::mutex> lk(m_pendingSearchMutex);
        if (m_pendingSearchHits.has_value()) {
            hits = std::move(*m_pendingSearchHits);
            m_pendingSearchHits.reset();
            hasResult = true;
        } else if (!m_pendingSearchError.isEmpty()) {
            error = m_pendingSearchError;
            m_pendingSearchError.clear();
            hasResult = true;
        }
    }

    if (!hasResult) {
        result.hasResult = false;
        return result;
    }

    m_searchLoading = false;
    result.hasResult = true;
    result.loading   = false;

    if (!error.isEmpty()) {
        result.error = error;
        return result;
    }

    /* Store paths and JSON for preview matching */
    m_currentSearchFilenames.clear();
    m_currentSearchJsons.clear();

    QVariantList resultList;
    for (const SearchHit &h : hits) {
        m_currentSearchFilenames.append(h.fullPath);

        QJsonObject map;
        map[QStringLiteral("name")]               = h.title;
        map[QStringLiteral("filename")]            = h.filename;
        map[QStringLiteral("group_path")]          = h.groupPath;
        map[QStringLiteral("full_path")]           = h.fullPath;
        map[QStringLiteral("snippet")]             = h.snippet;
        map[QStringLiteral("query")]               = m_searchQuery;
        map[QStringLiteral("created_at")]          = h.createdAt;
        map[QStringLiteral("updated_at")]          = h.updatedAt;
        map[QStringLiteral("block_count")]         = h.blockCount;
        map[QStringLiteral("preview_blocks_json")] = h.previewJson;

        QString jsonStr = QString::fromUtf8(
            QJsonDocument(map).toJson(QJsonDocument::Compact));
        m_currentSearchJsons.append(jsonStr);
        resultList.append(jsonStr);
    }

    m_searchResults = resultList;
    result.results  = resultList;
    return result;
}

/* ================================================================== */
/*  poll_search_previews() — collect background preview data           */
/* ================================================================== */

SearchPreviewPollResult SearchManager::poll_search_previews()
{
    SearchPreviewPollResult result;

    QMap<QString, QString> previews;
    bool hasPreviews = false;

    {
        std::lock_guard<std::mutex> lk(m_pendingPreviewMutex);
        if (m_pendingPreviews.has_value()) {
            previews = std::move(*m_pendingPreviews);
            m_pendingPreviews.reset();
            hasPreviews = true;
        }
    }

    if (!hasPreviews) {
        result.hasResult = false;
        return result;
    }

    /* Rebuild search results with preview data */
    QVariantList list;
    for (int i = 0; i < m_currentSearchFilenames.size(); ++i) {
        const QString &filename = m_currentSearchFilenames[i];
        if (i >= m_currentSearchJsons.size()) continue;

        QString jsonStr = m_currentSearchJsons[i];
        if (previews.contains(filename)) {
            QJsonObject obj = QJsonDocument::fromJson(jsonStr.toUtf8())
                                  .object();
            obj[QStringLiteral("preview_blocks_json")] = previews[filename];
            jsonStr = QString::fromUtf8(
                QJsonDocument(obj).toJson(QJsonDocument::Compact));
        }
        list.append(jsonStr);
    }

    if (!list.isEmpty()) {
        m_searchResults = list;
        result.hasResult = true;
        result.results   = list;
    }
    return result;
}

/* ================================================================== */
/*  get_linkable_pages_json() — link picker / autocomplete             */
/* ================================================================== */

QString SearchManager::get_linkable_pages_json(const QString &query)
{
    void *conn = m_ctx.rawConn();
    if (!conn) return QStringLiteral("[]");

    char *json = notes_core_recent_pages_json(conn, 1000);
    QString allPages = ffiStringToQString(json);

    if (query.trimmed().isEmpty())
        return allPages;

    /* Client-side filter on title / filename */
    QJsonDocument doc = QJsonDocument::fromJson(allPages.toUtf8());
    QJsonArray arr = doc.array();
    QJsonArray filtered;
    const QString q = query.trimmed().toLower();

    for (const QJsonValue &val : arr) {
        QJsonObject obj = val.toObject();
        const QString title    = obj[QStringLiteral("title")].toString().toLower();
        const QString filename = obj[QStringLiteral("filename")].toString().toLower();
        if (title.contains(q) || filename.contains(q))
            filtered.append(val);
    }

    return QString::fromUtf8(
        QJsonDocument(filtered).toJson(QJsonDocument::Compact));
}
