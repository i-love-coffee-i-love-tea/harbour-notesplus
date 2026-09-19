/* MainPageLoader.h — Main page data loading and background tree rebuild.
 *
 * Handles loading main page data (recent pages, journal lines, tree) and
 * background tree rebuild.  NOT a QObject — the caller owns the signal
 * target and polling lifecycle.
 *
 * Extracted from NotesBridge to keep the bridge thin.
 */

#ifndef MAINPAGELOADER_H
#define MAINPAGELOADER_H

#include <QVariantList>
#include <QString>
#include <QStringList>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QtConcurrent/QtConcurrent>
#include <mutex>
#include <optional>

#include "BridgeContext.h"

class MainPageLoader
{
public:
    /* Result of poll_main_page_data(). */
    struct PollResult {
        bool        hasResult      = false;
        QVariantList recentPages;
        QString     groupedTreeJson;
    };

    explicit MainPageLoader(const BridgeContext &ctx);

    /* Load journal lines and recent pages synchronously (FFI calls).
     * Updates m_recentJournalLines, m_journalBlocks, m_recentPages. */
    void loadMainPageDataSync();

    /* Convenience: loadMainPageDataSync() then rebuildTreeInBackground(). */
    void load_main_page_data(QObject *signalTarget);

    /* Consume pending MainPageData written by the background thread.
     * Returns PollResult with hasResult=false if nothing is pending. */
    PollResult poll_main_page_data();

    /* Build group tree + recent pages in a background thread.
     * Writes result to pending slot, then calls poll_main_page_data
     * on signalTarget via QMetaObject::invokeMethod. */
    void rebuildTreeInBackground(QObject *signalTarget);

    /* Store group display depth for rebuildTreeInBackground. */
    void setGroupDisplayDepth(int depth);

    /* Accessors for state updated by loadMainPageDataSync / poll. */
    const QVariantList &recentPages() const        { return m_recentPages; }
    const QString      &groupedTreeJson() const     { return m_groupedTreeJson; }
    const QVariantList &recentJournalLines() const  { return m_recentJournalLines; }
    const QVariantList &journalBlocks() const       { return m_journalBlocks; }

private:
    struct MainPageData {
        QStringList recentPageJsons;
        QString     groupedTreeJson;
    };

    const BridgeContext &m_ctx;

    /* State updated by loadMainPageDataSync / poll_main_page_data */
    QVariantList m_recentPages;
    QString      m_groupedTreeJson;
    QVariantList m_recentJournalLines;
    QVariantList m_journalBlocks;

    /* Background rebuild depth */
    int m_groupDisplayDepth = 2;

    /* Pending data from background thread */
    std::mutex                    m_pendingMainMutex;
    std::optional<MainPageData>   m_pendingMainPage;
};

#endif /* MAINPAGELOADER_H */
