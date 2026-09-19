/* PageStore.cpp — Page CRUD and current page state for Notes++.
 *
 * Block editing has been extracted to BlockEditor.
 *
 * All FFI calls are copied verbatim from NotesBridge.cpp, substituting
 * m_ctx.rawConn() for rawConn(), m_ctx.notesPath for m_notesPath, etc.
 * Background operations use QtConcurrent::run and write to a mutex-
 * protected slot, polled from the main thread by poll_results().
 */

#include "PageStore.h"

#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>

#include "../ffi/ffi_raii.h"

/* ================================================================== */
/*  Static helpers                                                     */
/* ================================================================== */

QVariantList PageStore::jsonArrayToQStringVariantList(const QString &jsonStr)
{
    QVariantList list;
    QJsonDocument doc = QJsonDocument::fromJson(jsonStr.toUtf8());
    if (!doc.isArray())
        return list;
    for (const QJsonValue &val : doc.array()) {
        QJsonObject obj = val.toObject();
        list.append(QString::fromUtf8(
            QJsonDocument(obj).toJson(QJsonDocument::Compact)));
    }
    return list;
}

/* ================================================================== */
/*  Constructor                                                        */
/* ================================================================== */

PageStore::PageStore(const BridgeContext &ctx, BlockListModel *model)
    : m_ctx(ctx)
    , m_blockListModel(model)
    , m_pathResolver(ctx.notesPath)
{
}

/* ================================================================== */
/*  Page operations                                                    */
/* ================================================================== */

void PageStore::load_page(const QString &name, QObject *signalTarget)
{
    QString fullPath = m_pathResolver.resolve(name, m_currentPageGroupPath);

    int lastSlash = fullPath.lastIndexOf(QLatin1Char('/'));
    QString groupName, pageName;
    if (lastSlash >= 0) {
        groupName = fullPath.left(lastSlash);
        pageName  = fullPath.mid(lastSlash + 1);
    } else {
        pageName = fullPath;
    }

    char *titleRaw = notes_core_page_extract_title(
        qstrToFFI(pageName), qstrToFFI(pageName));
    QString title = ffiStringToQString(titleRaw);
    if (title.isEmpty())
        title = PagePathResolver::extractTitle(pageName);

    m_currentPageName      = title;
    m_currentPageGroupPath = groupName;
    m_currentPageFullPath  = fullPath;
    m_currentPageFilePath  = fullPath;
    m_isJournalPage        = (fullPath == QLatin1String("journal.adoc"));

    m_loading = true;

    const std::string notesDir = m_ctx.notesPath.toStdString();
    const std::string path     = fullPath.toStdString();
    const bool drop            = m_ctx.dropComments();
    const std::string theme    = m_ctx.themeColorsJson().toStdString();
    const std::string opts     = m_ctx.buildOptionsJson().toStdString();

    auto alive = m_ctx.alive;
    QtConcurrent::run([this, alive, signalTarget, notesDir, path, drop,
                       theme, opts]() {
        PendingPageResult result;

        char *source = notes_core_page_get_source(
            notesDir.c_str(), path.c_str());
        QString content = ffiStringToQString(source);

        if (content.isEmpty()) {
            result.error = QStringLiteral("Page '%1' not found")
                               .arg(QString::fromStdString(path));
        } else {
            char *blocks = notes_core_page_parse_and_render_blocks_json(
                content.toUtf8().constData(),
                notesDir.c_str(),
                drop ? 1 : 0,
                theme.empty() ? nullptr : theme.c_str(),
                opts.empty()  ? nullptr : opts.c_str());
            result.blocks = jsonArrayToQStringVariantList(
                ffiStringToQString(blocks));
        }

        if (!alive->load(std::memory_order_acquire)) return;
        {
            std::lock_guard<std::mutex> lk(m_pendingMutex);
            m_pendingPage = std::move(result);
        }

        QMetaObject::invokeMethod(signalTarget, "poll_results",
                                  Qt::QueuedConnection);
    });
}

void PageStore::append_to_current_page(const QString &text, bool is_task,
                                       QObject *signalTarget)
{
    const QString trimmed = text.trimmed();
    if (trimmed.isEmpty()) return;

    const bool isJournal = m_isJournalPage
        || m_currentPageName.compare(QLatin1String("Journal"),
                                     Qt::CaseInsensitive) == 0;
    if (isJournal) {
        int rc = notes_core_journal_append(
            m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath),
            qstrToFFI(trimmed), is_task ? 1 : 0);
        if (rc < 0) {
            m_ctx.reportError(QStringLiteral("Failed to append to journal"));
            return;
        }
        load_page(QStringLiteral("journal.adoc"), signalTarget);
        return;
    }

    const QString fullP = m_pathResolver.currentRelative(
        m_currentPageName, m_currentPageGroupPath,
        m_currentPageFullPath, m_isJournalPage);
    char *src = notes_core_page_get_source(
        qstrToFFI(m_ctx.notesPath), qstrToFFI(fullP));
    QString content = ffiStringToQString(src);

    if (!content.isEmpty() && !content.endsWith(QLatin1Char('\n')))
        content += QLatin1Char('\n');
    content += trimmed;
    content += QLatin1Char('\n');

    int rc = notes_core_page_save_source(
        m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath), qstrToFFI(fullP),
        qstrToFFI(content));
    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to append to note"));
        return;
    }
    load_page(fullP, signalTarget);
}

QString PageStore::get_page_source(const QString &name)
{
    const QString fullPath = m_pathResolver.resolve(name, m_currentPageGroupPath);
    char *src = notes_core_page_get_source(
        qstrToFFI(m_ctx.notesPath), qstrToFFI(fullPath));
    return ffiStringToQString(src);
}

void PageStore::save_page_source(const QString &name, const QString &content,
                                 QObject *signalTarget)
{
    const QString fullPath = m_pathResolver.resolve(name, m_currentPageGroupPath);

    int rc = notes_core_page_save_source(
        m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath), qstrToFFI(fullPath),
        qstrToFFI(content));
    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to save page source"));
        return;
    }
    load_page(fullPath, signalTarget);
}

void PageStore::create_page(const QString &name, const QString &color, QObject *signalTarget)
{
    const QByteArray colorUtf8 = color.toUtf8();
    const char *colorPtr = color.isEmpty() ? nullptr : colorUtf8.constData();
    int rc = notes_core_page_create(
        m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath), qstrToFFI(name), colorPtr);
    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to create page: %1").arg(name));
        return;
    }

    if (m_rebuildTreeCallback)
        m_rebuildTreeCallback();

    QString fullPath = name;
    if (!fullPath.endsWith(QLatin1String(".adoc")))
        fullPath += QLatin1String(".adoc");
    load_page(fullPath, signalTarget);
}

bool PageStore::set_page_color(const QString &name, const QString &color)
{
    const QByteArray colorUtf8 = color.toUtf8();
    const char *colorPtr = color.isEmpty() ? nullptr : colorUtf8.constData();
    int rc = notes_core_page_set_color(
        m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath), qstrToFFI(name), colorPtr);
    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to set color for page: %1").arg(name));
        return false;
    }

    if (m_rebuildTreeCallback)
        m_rebuildTreeCallback();
    return true;
}

void PageStore::delete_page(const QString &name, QObject * /*signalTarget*/)
{
    const QString fullPath = m_pathResolver.resolve(name, m_currentPageGroupPath);
    const bool isCurrent =
        (m_currentPageName == name)
        || (m_currentPageFullPath == name)
        || (m_currentPageFullPath == fullPath);

    int rc = notes_core_page_delete(
        m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath), qstrToFFI(name));
    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to delete page: %1").arg(name));
        return;
    }

    if (isCurrent) {
        m_currentPageName.clear();
        m_currentPageGroupPath.clear();
        m_currentPageFullPath.clear();
        m_currentPageFilePath.clear();
        m_currentBlocks.clear();
        m_isJournalPage = false;
        m_blocksVersion = 0;
        m_loading = false;
        if (m_blockListModel)
            m_blockListModel->clear();
    }

    if (m_rebuildTreeCallback)
        m_rebuildTreeCallback();
}

bool PageStore::rename_page(const QString &old_path,
                            const QString &new_title,
                            QObject *signalTarget)
{
    int rc = notes_core_page_rename(
        m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath),
        qstrToFFI(old_path), qstrToFFI(new_title));
    if (rc < 0) {
        m_ctx.reportError(
            QStringLiteral("Failed to rename page: %1").arg(old_path));
        return false;
    }

    const QString fullPath = m_pathResolver.resolve(old_path, m_currentPageGroupPath);
    if (m_currentPageFullPath == fullPath || m_currentPageName == old_path) {
        load_page(new_title, signalTarget);
    }

    if (m_rebuildTreeCallback)
        m_rebuildTreeCallback();
    return true;
}

void PageStore::navigate_to_page(const QString &name, QObject *signalTarget)
{
    load_page(name, signalTarget);
}

/* ================================================================== */
/*  Index                                                              */
/* ================================================================== */

QString PageStore::rebuild_index()
{
    void *conn = m_ctx.rawConn();
    if (!conn)
        return QStringLiteral("Error: database not ready");

    int rc = notes_core_rebuild_index(conn, qstrToFFI(m_ctx.notesPath));
    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to rebuild index"));
        return QStringLiteral("Error: rebuild failed");
    }

    if (m_rebuildTreeCallback)
        m_rebuildTreeCallback();
    return QStringLiteral("Index rebuilt");
}

/* ================================================================== */
/*  Path update (used by move_page_to_group)                           */
/* ================================================================== */

void PageStore::updatePagePaths(const QString &groupPath,
                                const QString &fullPath,
                                const QString &filePath)
{
    m_currentPageGroupPath = groupPath;
    m_currentPageFullPath  = fullPath;
    m_currentPageFilePath  = filePath;
}

/* ================================================================== */
/*  Async poll                                                         */
/* ================================================================== */

PollResult PageStore::poll_results()
{
    PollResult result;
    {
        std::lock_guard<std::mutex> lk(m_pendingMutex);
        if (m_pendingPage.has_value()) {
            PendingPageResult pending = std::move(*m_pendingPage);
            m_pendingPage.reset();
            result.hasResult = true;
            result.blocks    = std::move(pending.blocks);
            result.error     = std::move(pending.error);
        }
    }

    if (!result.hasResult)
        return result;

    m_loading = false;

    if (!result.error.isEmpty()) {
        m_ctx.reportError(result.error);
        return result;
    }

    m_currentBlocks = result.blocks;
    m_blocksVersion++;
    if (m_blockListModel)
        m_blockListModel->setBlocks(m_currentBlocks);
    return result;
}
