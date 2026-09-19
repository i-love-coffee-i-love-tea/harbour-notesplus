/* SearchPreviewGenerator.cpp — Background preview block generation for
 * search results in Notes++.
 *
 * Implementation of the SearchPreviewGenerator class.  Each call to
 * generatePreviews() spawns a background thread (QtConcurrent::run)
 * that reads page source via FFI, parses blocks, and delivers the
 * merged preview map back to the Qt main thread through signalTarget.
 */

#include "SearchPreviewGenerator.h"

#include "../ffi/ffi_raii.h"

/* ================================================================== */
/*  Constructor                                                        */
/* ================================================================== */

SearchPreviewGenerator::SearchPreviewGenerator(const BridgeContext &ctx)
    : m_ctx(ctx)
{
}

/* ================================================================== */
/*  generatePreviews() — kick off background preview generation        */
/* ================================================================== */

void SearchPreviewGenerator::generatePreviews(const QList<SearchHit> &hits,
                                              int generation,
                                              QObject *signalTarget)
{
    m_generation.store(generation, std::memory_order_relaxed);

    /* Capture everything the lambda needs by value */
    auto alive2        = m_ctx.alive;
    auto notesPath     = m_ctx.notesPath;
    auto dropCommentsFn = m_ctx.dropComments;
    int gen            = generation;

    QtConcurrent::run([this, alive2, gen, signalTarget,
                       notesPath, dropCommentsFn, hits]() {
        if (gen != m_generation.load(std::memory_order_relaxed)) return;
        if (!alive2->load(std::memory_order_acquire)) return;

        const std::string nd = notesPath.toStdString();
        const bool drop = dropCommentsFn();

        QMap<QString, QString> previewMap;
        for (const SearchHit &h : hits) {
            if (gen != m_generation.load(std::memory_order_relaxed)) return;

            const std::string fp = h.fullPath.toStdString();
            char *src = notes_core_page_get_source(nd.c_str(), fp.c_str());
            QString content = ffiStringToQString(src);
            if (content.isEmpty()) continue;

            char *blocks = notes_core_parse_blocks_json(
                content.toUtf8().constData(), drop ? 1 : 0);
            previewMap[h.fullPath] = ffiStringToQString(blocks);
        }

        if (gen != m_generation.load(std::memory_order_relaxed)) return;

        {
            std::lock_guard<std::mutex> lk(m_pendingPreviewMutex);
            m_pendingPreviews = std::move(previewMap);
        }

        QMetaObject::invokeMethod(signalTarget, "poll_search_previews",
                                  Qt::QueuedConnection);
    });
}

/* ================================================================== */
/*  storeSearchContext() — save filenames / JSON from poll_search       */
/* ================================================================== */

void SearchPreviewGenerator::storeSearchContext(
    const QStringList &filenames, const QStringList &jsons)
{
    m_currentSearchFilenames = filenames;
    m_currentSearchJsons     = jsons;
}

/* ================================================================== */
/*  poll_previews() — collect background preview data on main thread    */
/* ================================================================== */

SearchPreviewPollResult SearchPreviewGenerator::poll_previews()
{
    SearchPreviewPollResult result;

    QMap<QString, QString> previews;
    bool hasPreviews = false;

    {
        std::lock_guard<std::mutex> lk(m_pendingPreviewMutex);
        if (m_pendingPreviews.has_value()) {
            previews    = std::move(*m_pendingPreviews);
            hasPreviews = true;
            m_pendingPreviews.reset();
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
        result.hasResult = true;
        result.results   = list;
    }
    return result;
}

/* ================================================================== */
/*  clear() — cancel in-flight previews, reset pending state           */
/* ================================================================== */

void SearchPreviewGenerator::clear()
{
    m_generation.fetch_add(1, std::memory_order_relaxed);
    std::lock_guard<std::mutex> lk(m_pendingPreviewMutex);
    m_pendingPreviews.reset();
}
