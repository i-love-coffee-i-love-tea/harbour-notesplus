/* MainPageLoader.cpp — Main page data loading and background tree rebuild.
 *
 * FFI calls are copied verbatim from NotesBridge.cpp to preserve exact
 * behaviour: notes_core_journal_recent_lines, notes_core_recent_pages_json,
 * notes_core_build_group_tree_json.
 */

#include "MainPageLoader.h"

#include "../ffi/ffi_raii.h"

/* ================================================================== */
/*  Constructor                                                        */
/* ================================================================== */

MainPageLoader::MainPageLoader(const BridgeContext &ctx)
    : m_ctx(ctx)
    , m_groupedTreeJson(QStringLiteral("[]"))
{
}

/* ================================================================== */
/*  setGroupDisplayDepth                                                */
/* ================================================================== */

void MainPageLoader::setGroupDisplayDepth(int depth)
{
    m_groupDisplayDepth = depth;
}

/* ================================================================== */
/*  loadMainPageDataSync — synchronous FFI calls                       */
/* ================================================================== */

void MainPageLoader::loadMainPageDataSync()
{
    /* Journal data — fast synchronous FFI calls */
    char *lines = notes_core_journal_recent_lines(
        qstrToFFI(m_ctx.notesPath), 5);
    QString linesStr = ffiStringToQString(lines);

    m_recentJournalLines.clear();
    m_journalBlocks.clear();

    QJsonDocument jdoc = QJsonDocument::fromJson(linesStr.toUtf8());
    if (jdoc.isArray()) {
        for (const QJsonValue &v : jdoc.array())
            m_recentJournalLines.append(v.toString());
    }

    /* Get recent pages for the initial quick display */
    void *conn = m_ctx.rawConn();
    if (conn) {
        char *recent = notes_core_recent_pages_json(conn, 10);
        QString recentStr = ffiStringToQString(recent);
        QJsonDocument rdoc = QJsonDocument::fromJson(recentStr.toUtf8());
        if (rdoc.isArray()) {
            QVariantList list;
            for (const QJsonValue &v : rdoc.array()) {
                list.append(QString::fromUtf8(
                    QJsonDocument(v.toObject()).toJson(
                        QJsonDocument::Compact)));
            }
            m_recentPages = list;
        }
    }
}

/* ================================================================== */
/*  rebuildTreeInBackground — async tree + recent pages                */
/* ================================================================== */

void MainPageLoader::rebuildTreeInBackground(QObject *signalTarget)
{
    void *conn = m_ctx.rawConn();
    if (!conn)
        return;

    /* Snapshot all values the background thread needs */
    const int         depth        = m_groupDisplayDepth;
    const bool        dropComments = m_ctx.dropComments();
    const std::string notesDir     = m_ctx.notesPath.toStdString();
    const std::string themeJson    = m_ctx.themeColorsJson().toStdString();
    const std::string optsJson     = m_ctx.buildOptionsJson().toStdString();
    auto              alive        = m_ctx.alive;

    QtConcurrent::run([this, alive, conn, notesDir, depth, dropComments,
                       themeJson, optsJson, signalTarget]() {
        /* Build group tree */
        char *tree = notes_core_build_group_tree_json(
            conn, notesDir.c_str(), depth,
            dropComments ? 1 : 0,
            themeJson.empty() ? nullptr : themeJson.c_str(),
            optsJson.empty()  ? nullptr : optsJson.c_str());
        QString treeJson = ffiStringToQString(tree);

        /* Also refresh recent pages list (quick DB query) */
        char *recent = notes_core_recent_pages_json(conn, 10);
        QJsonDocument doc = QJsonDocument::fromJson(
            ffiStringToQString(recent).toUtf8());
        QStringList pageJsons;
        if (doc.isArray()) {
            for (const QJsonValue &v : doc.array()) {
                pageJsons.append(QString::fromUtf8(
                    QJsonDocument(v.toObject()).toJson(
                        QJsonDocument::Compact)));
            }
        }

        if (!alive->load(std::memory_order_acquire)) return;

        MainPageData data;
        data.recentPageJsons = pageJsons;
        data.groupedTreeJson = treeJson;

        {
            std::lock_guard<std::mutex> lk(m_pendingMainMutex);
            m_pendingMainPage = std::move(data);
        }

        QMetaObject::invokeMethod(signalTarget, "poll_main_page_data",
                                  Qt::QueuedConnection);
    });
}

/* ================================================================== */
/*  load_main_page_data — sync load + background rebuild               */
/* ================================================================== */

void MainPageLoader::load_main_page_data(QObject *signalTarget)
{
    loadMainPageDataSync();
    rebuildTreeInBackground(signalTarget);
}

/* ================================================================== */
/*  poll_main_page_data — consume pending background result            */
/* ================================================================== */

MainPageLoader::PollResult MainPageLoader::poll_main_page_data()
{
    MainPageData data;
    bool hasResult = false;

    {
        std::lock_guard<std::mutex> lk(m_pendingMainMutex);
        if (m_pendingMainPage.has_value()) {
            data = std::move(*m_pendingMainPage);
            m_pendingMainPage.reset();
            hasResult = true;
        }
    }

    PollResult result;
    result.hasResult = hasResult;

    if (!hasResult)
        return result;

    if (!data.recentPageJsons.isEmpty()) {
        QVariantList list;
        for (const QString &j : data.recentPageJsons)
            list.append(j);
        m_recentPages = list;
    }
    m_groupedTreeJson = data.groupedTreeJson;

    result.recentPages      = m_recentPages;
    result.groupedTreeJson  = m_groupedTreeJson;
    return result;
}
