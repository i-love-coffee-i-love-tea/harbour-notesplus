/* PageStore.cpp — Page CRUD, block editing, journal operations, and current
 * page state for Notes++.
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

/// Parse a JSON array string into a QVariantList where each element is a
/// compact JSON object string.  Matches the Rust bridge behaviour of
/// pushing each block as a QString-encoded JSON object into QVariantList.
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

/// Extract a title from a file path or name (strip directory + .adoc suffix).
QString PageStore::extractTitle(const QString &name)
{
    QString t = name.section(QLatin1Char('/'), -1);
    if (t.endsWith(QLatin1String(".adoc")))
        t.chop(5);
    return t;
}

/* ================================================================== */
/*  Constructor                                                        */
/* ================================================================== */

PageStore::PageStore(const BridgeContext &ctx, BlockListModel *model)
    : m_ctx(ctx)
    , m_blockListModel(model)
{
}

/* ================================================================== */
/*  Path resolution                                                    */
/* ================================================================== */

QString PageStore::resolvePagePath(const QString &name)
{
    if (name.compare(QLatin1String("journal.adoc"), Qt::CaseInsensitive) == 0
        || name.compare(QLatin1String("Journal"), Qt::CaseInsensitive) == 0)
        return QStringLiteral("journal.adoc");

    /* If it already contains a slash, assume it is a relative path */
    if (name.contains(QLatin1Char('/'))) {
        if (!name.endsWith(QLatin1String(".adoc")))
            return name + QLatin1String(".adoc");
        return name;
    }

    /* Try scoped under current group */
    if (!m_currentPageGroupPath.isEmpty()) {
        QString scoped = m_currentPageGroupPath + QLatin1Char('/') + name;
        if (!scoped.endsWith(QLatin1String(".adoc")))
            scoped += QLatin1String(".adoc");
        /* Verify file exists */
        QString absPath = m_ctx.notesPath + QLatin1Char('/') + scoped;
        if (QFile::exists(absPath))
            return scoped;
    }

    /* Fallback: name at root */
    if (!name.endsWith(QLatin1String(".adoc")))
        return name + QLatin1String(".adoc");
    return name;
}

QString PageStore::currentPageRelativePath()
{
    if (m_isJournalPage
        || m_currentPageName.compare(QLatin1String("Journal"),
                                     Qt::CaseInsensitive) == 0)
        return QStringLiteral("journal.adoc");
    if (!m_currentPageFullPath.isEmpty())
        return m_currentPageFullPath;
    if (!m_currentPageGroupPath.isEmpty() && !m_currentPageName.isEmpty()) {
        QString p = m_currentPageGroupPath + QLatin1Char('/') + m_currentPageName;
        if (!p.endsWith(QLatin1String(".adoc")))
            p += QLatin1String(".adoc");
        return p;
    }
    if (!m_currentPageName.isEmpty()) {
        if (!m_currentPageName.endsWith(QLatin1String(".adoc")))
            return m_currentPageName + QLatin1String(".adoc");
        return m_currentPageName;
    }
    return QString();
}

/* ================================================================== */
/*  Page operations                                                    */
/* ================================================================== */

void PageStore::load_page(const QString &name, QObject *signalTarget)
{
    /* Resolve page path */
    QString fullPath = resolvePagePath(name);

    /* Derive metadata from the resolved path */
    int lastSlash = fullPath.lastIndexOf(QLatin1Char('/'));
    QString groupName, pageName;
    if (lastSlash >= 0) {
        groupName = fullPath.left(lastSlash);
        pageName  = fullPath.mid(lastSlash + 1);
    } else {
        pageName = fullPath;
    }

    /* Extract a human-readable title */
    char *titleRaw = notes_core_page_extract_title(
        qstrToFFI(pageName), qstrToFFI(pageName));
    QString title = ffiStringToQString(titleRaw);
    if (title.isEmpty())
        title = extractTitle(pageName);

    m_currentPageName      = title;
    m_currentPageGroupPath = groupName;
    m_currentPageFullPath  = fullPath;
    m_currentPageFilePath  = fullPath;
    m_isJournalPage        = (fullPath == QLatin1String("journal.adoc"));

    /* Background: read source + parse blocks */
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
            /* Parse and render blocks */
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

void PageStore::save_block(int index, const QString &raw_text)
{
    save_block_range(index, 1, raw_text, nullptr);
}

void PageStore::save_block_range(int start_index, int count,
                                 const QString &raw_text,
                                 QObject *signalTarget)
{
    if (start_index < 0 || count < 0) return;

    const QString pagePath = currentPageRelativePath();
    if (pagePath.isEmpty()) return;

    int rc = notes_core_page_save_block(
        m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath), qstrToFFI(pagePath),
        start_index, count, qstrToFFI(raw_text),
        m_ctx.dropComments() ? 1 : 0);

    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to save block"));
        return;
    }

    /* Optimistic update: read back, parse, update UI immediately */
    char *source = notes_core_page_get_source(
        qstrToFFI(m_ctx.notesPath), qstrToFFI(pagePath));
    QString content = ffiStringToQString(source);
    if (!content.isEmpty()) {
        char *blocks = notes_core_page_parse_and_render_blocks_json(
            content.toUtf8().constData(),
            qstrToFFI(m_ctx.notesPath),
            m_ctx.dropComments() ? 1 : 0,
            m_ctx.themeColorsJson().isEmpty() ? nullptr
                                              : qstrToFFI(m_ctx.themeColorsJson()),
            qstrToFFI(m_ctx.buildOptionsJson()));
        m_currentBlocks = jsonArrayToQStringVariantList(
            ffiStringToQString(blocks));
        if (m_blockListModel) {
            m_blockListModel->setBlocks(m_currentBlocks);
        }
        m_blocksVersion++;
    }

    /* Background reload for full refresh */
    if (signalTarget)
        load_page(pagePath, signalTarget);
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

    /* Regular page: read, append, save */
    const QString fullP = currentPageRelativePath();
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

void PageStore::save_journal_block(int index, const QString &raw_text)
{
    if (index < 0) return;

    int rc = notes_core_page_save_block(
        m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath), "journal.adoc",
        index, 1, qstrToFFI(raw_text), m_ctx.dropComments() ? 1 : 0);

    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to save journal block"));
        return;
    }

    if (m_rebuildTreeCallback)
        m_rebuildTreeCallback();
}

void PageStore::toggle_journal_checkbox(int block_index,
                                        const QString &item_path)
{
    if (block_index < 0) return;

    int rc = notes_core_page_toggle_checkbox(
        m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath), "journal.adoc",
        block_index, qstrToFFI(item_path));

    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to toggle journal checkbox"));
        return;
    }

    if (m_rebuildTreeCallback)
        m_rebuildTreeCallback();
}

void PageStore::append_to_journal(const QString &text, bool is_task)
{
    const QString trimmed = text.trimmed();
    if (trimmed.isEmpty()) return;

    int rc = notes_core_journal_append(
        m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath),
        qstrToFFI(trimmed), is_task ? 1 : 0);
    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to append to journal"));
        return;
    }

    if (m_rebuildTreeCallback)
        m_rebuildTreeCallback();
}

QString PageStore::get_page_source(const QString &name)
{
    const QString fullPath = resolvePagePath(name);
    char *src = notes_core_page_get_source(
        qstrToFFI(m_ctx.notesPath), qstrToFFI(fullPath));
    return ffiStringToQString(src);
}

void PageStore::save_page_source(const QString &name, const QString &content,
                                 QObject *signalTarget)
{
    const QString fullPath = resolvePagePath(name);

    int rc = notes_core_page_save_source(
        m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath), qstrToFFI(fullPath),
        qstrToFFI(content));
    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to save page source"));
        return;
    }
    load_page(fullPath, signalTarget);
}

void PageStore::create_page(const QString &name, QObject *signalTarget)
{
    int rc = notes_core_page_create(
        m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath), qstrToFFI(name));
    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to create page: %1").arg(name));
        return;
    }

    if (m_rebuildTreeCallback)
        m_rebuildTreeCallback();

    /* Navigate to the new page */
    QString fullPath = name;
    if (!fullPath.endsWith(QLatin1String(".adoc")))
        fullPath += QLatin1String(".adoc");
    load_page(fullPath, signalTarget);
}

void PageStore::delete_page(const QString &name, QObject * /*signalTarget*/)
{
    /* Determine if the page being deleted is the currently-viewed one */
    const QString fullPath = resolvePagePath(name);
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
        m_currentBlocks.clear();
        if (m_blockListModel) {
            m_blockListModel->clear();
        }
        m_isJournalPage = false;
        m_currentPageFilePath.clear();
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

    /* If the renamed page is currently loaded, reload it */
    const QString fullPath = resolvePagePath(old_path);
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

void PageStore::insert_link_at_cursor(int block_idx, int /*cursor_pos*/,
                                      const QString &target,
                                      QObject *signalTarget)
{
    if (block_idx < 0) return;

    const QString pagePath = currentPageRelativePath();
    if (pagePath.isEmpty()) return;

    /* Read current source */
    char *src = notes_core_page_get_source(
        qstrToFFI(m_ctx.notesPath), qstrToFFI(pagePath));
    QString content = ffiStringToQString(src);
    if (content.isEmpty()) return;

    /* Parse blocks to find the target block */
    char *blocksJson = notes_core_parse_blocks_json(
        qstrToFFI(content), m_ctx.dropComments() ? 1 : 0);
    QJsonDocument doc = QJsonDocument::fromJson(
        ffiStringToQString(blocksJson).toUtf8());
    QJsonArray arr = doc.array();

    if (block_idx >= arr.size()) return;

    /* Build xref link */
    const QString xref = QStringLiteral("xref:%1.adoc[%1]").arg(target);

    /* Get block's text and append the link */
    QJsonObject blockObj = arr[block_idx].toObject();
    QString existingText = blockObj[QStringLiteral("text")].toString();
    if (existingText.isEmpty())
        existingText = blockObj[QStringLiteral("raw")].toString();
    QString newRaw = existingText + QLatin1Char(' ') + xref;

    /* Replace the block */
    int rc = notes_core_page_save_block(
        m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath), qstrToFFI(pagePath),
        block_idx, 1, qstrToFFI(newRaw), m_ctx.dropComments() ? 1 : 0);

    if (rc >= 0)
        load_page(pagePath, signalTarget);
}

void PageStore::toggle_checkbox(int block_index, const QString &item_path,
                                QObject *signalTarget)
{
    if (block_index < 0) return;

    const QString pagePath = currentPageRelativePath();
    if (pagePath.isEmpty()) return;

    int rc = notes_core_page_toggle_checkbox(
        m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath), qstrToFFI(pagePath),
        block_index, qstrToFFI(item_path));

    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to toggle checkbox"));
        return;
    }
    if (m_blockListModel) {
        m_blockListModel->toggleCheckbox(block_index, item_path);
    }
    load_page(pagePath, signalTarget);
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
    if (m_blockListModel) {
        m_blockListModel->setBlocks(m_currentBlocks);
    }
    m_blocksVersion++;
    return result;
}
