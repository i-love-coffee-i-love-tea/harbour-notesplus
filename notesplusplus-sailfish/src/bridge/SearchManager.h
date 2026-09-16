/* SearchManager.h — FTS search, background search execution, preview
 * generation, and linkable-pages lookup for Notes++.
 *
 * Extracted from NotesBridge as a non-QObject domain class.  The facade
 * (NotesBridge) owns the SearchManager, and handles signal emission.
 * do_search takes a QObject* signalTarget whose "poll_search" /
 * "poll_search_previews" slots are invoked when background work completes.
 *
 * SearchManager creates its own DB connections per query via
 * notes_core_search_new() + notes_core_search_start().  It does NOT use
 * the shared conn from BridgeContext.
 */

#ifndef SEARCHMANAGER_H
#define SEARCHMANAGER_H

#include <QString>
#include <QStringList>
#include <QVariantList>
#include <QMap>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QtConcurrent/QtConcurrent>
#include <mutex>
#include <optional>
#include <atomic>
#include <thread>
#include <chrono>

#include "BridgeContext.h"

/* ---- Result structs returned by poll methods ---- */

struct SearchPollResult {
    bool         hasResult = false;
    QVariantList results;
    bool         loading   = false;
    QString      error;
};

struct SearchPreviewPollResult {
    bool         hasResult = false;
    QVariantList results;
};

/* ---- Internal hit record used during background search ---- */

struct SearchHit {
    QString title, filename, groupPath, fullPath, snippet;
    QString createdAt, updatedAt;
    int     blockCount = 0;
    QString previewJson;
};

class SearchManager
{
public:
    explicit SearchManager(const BridgeContext &ctx);

    /* ---- Search operations ---- */

    /// Start an async FTS search.  Results are delivered to signalTarget's
    /// "poll_search" slot.  Preview data follows via "poll_search_previews".
    void do_search(const QString &query, QObject *signalTarget);

    /// Clear search state (synchronous).  Called when the user changes the
    /// query text but doesn't want to trigger a full search yet.
    void search(const QString &query);

    /// Called on the main thread to collect background search results.
    SearchPollResult poll_search();

    /// Called on the main thread to collect background preview data.
    SearchPreviewPollResult poll_search_previews();

    /// Return linkable pages as a JSON array string.  If query is non-empty,
    /// results are client-side filtered on title / filename.
    QString get_linkable_pages_json(const QString &query);

    /* ---- Accessors for facade ---- */

    QString      searchQuery()   const { return m_searchQuery; }
    QVariantList searchResults() const { return m_searchResults; }
    bool         searchLoading() const { return m_searchLoading; }

private:
    /* ---- DB path helper (same logic as NotesBridge) ---- */
    static QString dbPathFor(const QString &dataDir);

    const BridgeContext &m_ctx;

    /* ---- Search state ---- */
    QString      m_searchQuery;
    QVariantList m_searchResults;
    bool         m_searchLoading = false;

    /* ---- Pending search result (background -> poll_search) ---- */
    std::mutex                         m_pendingSearchMutex;
    std::optional<QList<SearchHit>>    m_pendingSearchHits;
    QString                            m_pendingSearchError;
    std::atomic<int>                   m_searchGeneration{0};

    /* ---- Pending search previews (background -> poll_search_previews) ---- */
    std::mutex                               m_pendingPreviewMutex;
    std::optional<QMap<QString, QString>>    m_pendingPreviews;

    /* ---- Search state for preview matching ---- */
    QStringList m_currentSearchFilenames;
    QStringList m_currentSearchJsons;

    Q_DISABLE_COPY(SearchManager)
};

#endif /* SEARCHMANAGER_H */
