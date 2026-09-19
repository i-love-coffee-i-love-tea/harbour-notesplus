/* SearchPreviewGenerator.h — Background preview block generation for
 * search results in Notes++.
 *
 * Extracted from SearchManager.  Generates preview block JSON for each
 * search hit on a background thread (QtConcurrent::run), delivering
 * results back to the main thread via signalTarget's
 * "poll_search_previews" slot.
 */

#ifndef SEARCHPREVIEWGENERATOR_H
#define SEARCHPREVIEWGENERATOR_H

#include <QString>
#include <QStringList>
#include <QMap>
#include <QtConcurrent/QtConcurrent>
#include <mutex>
#include <optional>
#include <atomic>

#include "BridgeContext.h"
#include "SearchManager.h"   /* SearchHit, SearchPreviewPollResult */

class SearchPreviewGenerator
{
public:
    explicit SearchPreviewGenerator(const BridgeContext &ctx);

    /// Start background preview generation for the given hits.
    /// signalTarget's poll_search_previews slot is invoked when done.
    void generatePreviews(const QList<SearchHit> &hits, int generation,
                          QObject *signalTarget);

    /// Store the filenames / JSON strings produced by poll_search so that
    /// poll_previews can merge preview data into them.
    void storeSearchContext(const QStringList &filenames,
                           const QStringList &jsons);

    /// Collect preview results (called on main thread).
    SearchPreviewPollResult poll_previews();

    /// Clear pending state and cancel any in-flight preview work.
    void clear();

private:
    const BridgeContext &m_ctx;

    /* ---- Pending search previews (background -> poll_previews) ---- */
    std::mutex                              m_pendingPreviewMutex;
    std::optional<QMap<QString, QString>>   m_pendingPreviews;

    /* ---- Search result context for preview matching ---- */
    QStringList m_currentSearchFilenames;
    QStringList m_currentSearchJsons;

    /* ---- Generation counter for staleness checks ---- */
    std::atomic<int> m_generation{0};

    Q_DISABLE_COPY(SearchPreviewGenerator)
};

#endif /* SEARCHPREVIEWGENERATOR_H */
