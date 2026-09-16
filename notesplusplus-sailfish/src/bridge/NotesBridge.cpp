/* NotesBridge.cpp — C++ QObject bridge for Notes++ core functionality.
 *
 * Implementation of the NotesBridge class, porting the Rust NotesBridge
 * (mod.rs + pages.rs + server_bridge.rs + journal_bridge.rs + search_bridge.rs)
 * to C++ using the FFI in notesplusplus_core.h.
 *
 * Each Q_INVOKABLE method calls the corresponding FFI function via the RAII
 * wrappers defined in ffi_raii.h.  Background operations (load_page, search,
 * main_page_data) use std::thread + mutex-protected result slots, polled from
 * the Qt main thread by the poll_* methods.
 */

#include "NotesBridge.h"

#include <QDir>
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>
#include <QThread>
#include <QStandardPaths>
#include <cstdlib>

/* ================================================================== */
/*  Static helpers                                                     */
/* ================================================================== */

/// Parse a JSON array string into a QVariantList where each element is a
/// compact JSON object string.  This matches the Rust bridge behaviour of
/// pushing each block as a QString-encoded JSON object into QVariantList.
static QVariantList jsonArrayToQStringVariantList(const QString &jsonStr)
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
static QString extractTitle(const QString &name)
{
    QString t = name.section(QLatin1Char('/'), -1);
    if (t.endsWith(QLatin1String(".adoc")))
        t.chop(5);
    return t;
}

/// Return the default DB filename from the FFI constants (freed automatically).
static QString defaultDbFilename()
{
    return ffiStringToQString(notes_core_const_db_filename());
}

/// Build the db_path as a QString.
static QString dbPathFor(const QString &dataDir)
{
    return dataDir + QLatin1Char('/') + defaultDbFilename();
}

/* ================================================================== */
/*  Constructor / Destructor                                           */
/* ================================================================== */

NotesBridge::NotesBridge(QObject *parent)
    : QObject(parent)
    , paths_(notes_core_app_paths_new())
    , m_blockListModel(new BlockListModel(this))
{
    m_notesPath = ffiStringToQString(
        notes_core_app_paths_notes_dir(paths_.get()));
    m_dataDir = ffiStringToQString(
        notes_core_app_paths_data_dir(paths_.get()));

    /* Default property values (mirror Rust Default::default()) */
    m_notesDir        = m_notesPath;
    m_groupedTreeJson = QStringLiteral("[]");
    m_bindAddress     = QStringLiteral("0.0.0.0");

    /* Kick off background DB init */
    ensureInit();
}

NotesBridge::~NotesBridge() = default;

/* ================================================================== */
/*  Internal helpers                                                   */
/* ================================================================== */

void *NotesBridge::rawConn() const
{
    return conn_.get();
}

QString NotesBridge::buildOptionsJson() const
{
    QJsonObject opts;
    opts[QStringLiteral("notes_dir")]            = m_notesPath;
    opts[QStringLiteral("allow_external_images")] = true;
    return QString::fromUtf8(
        QJsonDocument(opts).toJson(QJsonDocument::Compact));
}

void NotesBridge::reportError(const QString &msg)
{
    m_errorMessage = msg;
    emit error_occurred(msg);
}

/* ---- Initialisation (background + blocking) ---- */

void NotesBridge::ensureInit()
{
    std::lock_guard<std::mutex> lk(m_initMutex);
    if (m_initStarted || m_initDone)
        return;
    m_initStarted = true;

    /* Capture values for the thread */
    const std::string dataDir  = m_dataDir.toStdString();
    const std::string notesDir = m_notesPath.toStdString();
    const std::string dbPath   = dbPathFor(m_dataDir).toStdString();

    std::thread([this, dataDir, notesDir, dbPath]() {
        /* Create directories */
        QDir().mkpath(QString::fromStdString(notesDir));
        QDir().mkpath(QString::fromStdString(dataDir) + QStringLiteral("/exports"));

        /* Open database */
        void *raw = notes_core_db_open(dbPath.c_str());
        if (raw) {
            /* Rebuild index so the DB knows about all .adoc files */
            notes_core_rebuild_index(raw, notesDir.c_str());
        }

        {
            std::lock_guard<std::mutex> lk(m_initMutex);
            m_initResult = raw;
            m_initDone   = true;
        }
        m_initCv.notify_one();

        QMetaObject::invokeMethod(this, [this]() {
            if (this->pollInit()) {
                this->load_main_page_data();
            }
        }, Qt::QueuedConnection);
    }).detach();
}

bool NotesBridge::pollInit()
{
    if (conn_)
        return true;

    void *raw = nullptr;
    {
        std::lock_guard<std::mutex> lk(m_initMutex);
        if (!m_initDone)
            return false;
        raw = m_initResult;
        m_initResult = nullptr;          /* ownership transferred below */
    }

    if (raw) {
        conn_.reset(raw);
        m_initialized = true;
        emit initialized_changed();
        return true;
    }
    return false;
}

bool NotesBridge::ensureInitBlocking()
{
    if (conn_)
        return true;
    ensureInit();
    {
        std::unique_lock<std::mutex> lk(m_initMutex);
        m_initCv.wait(lk, [this]() { return m_initDone; });
    }
    return pollInit();
}

/* ---- Path resolution ---- */

QString NotesBridge::resolvePagePath(const QString &name)
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
        QString absPath = m_notesPath + QLatin1Char('/') + scoped;
        if (QFile::exists(absPath))
            return scoped;
    }

    /* Fallback: name at root */
    if (!name.endsWith(QLatin1String(".adoc")))
        return name + QLatin1String(".adoc");
    return name;
}

QString NotesBridge::currentPageRelativePath()
{
    if (m_isJournalPage
        || m_currentPageName.compare(QLatin1String("Journal"), Qt::CaseInsensitive) == 0)
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
/*  Background tree rebuild helper (shared by several methods)         */
/* ================================================================== */

void NotesBridge::rebuildTreeInBackground()
{
    if (!rawConn())
        return;

    const int  depth         = m_groupDisplayDepth;
    const bool dropComments  = m_dropComments;
    const std::string notesDir  = m_notesPath.toStdString();
    const std::string themeJson = m_themeColorsJson.toStdString();
    const std::string optsJson  = buildOptionsJson().toStdString();

    /* Take a snapshot of the connection pointer — the background thread
     * only reads from it, and conn_ lives for the entire app lifetime. */
    void *conn = rawConn();

    std::thread([this, conn, notesDir, depth, dropComments,
                 themeJson, optsJson]() {
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

        MainPageData data;
        data.recentPageJsons = pageJsons;
        data.groupedTreeJson = treeJson;

        {
            std::lock_guard<std::mutex> lk(m_pendingMainMutex);
            m_pendingMainPage = std::move(data);
        }

        QMetaObject::invokeMethod(this, [this]() {
            this->poll_main_page_data();
        }, Qt::QueuedConnection);
    }).detach();
}

/* ================================================================== */
/*  Page operations                                                    */
/* ================================================================== */

void NotesBridge::load_page(QString name)
{
    if (!ensureInitBlocking())
        return;

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
    emit current_page_group_path_changed();
    m_currentPageFullPath  = fullPath;
    emit current_page_full_path_changed();
    m_currentPageFilePath  = fullPath;
    m_isJournalPage        = (fullPath == QLatin1String("journal.adoc"));
    emit page_changed();

    /* Background: read source + parse blocks */
    m_isLoading = true;
    emit loading_changed();

    const std::string notesDir = m_notesPath.toStdString();
    const std::string path     = fullPath.toStdString();
    const bool drop            = m_dropComments;
    const std::string theme    = m_themeColorsJson.toStdString();
    const std::string opts     = buildOptionsJson().toStdString();

    std::thread([this, notesDir, path, drop, theme, opts]() {
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

        {
            std::lock_guard<std::mutex> lk(m_pendingMutex);
            m_pendingPage = std::move(result);
        }

        QMetaObject::invokeMethod(this, [this]() {
            this->poll_results();
        }, Qt::QueuedConnection);
    }).detach();
}

void NotesBridge::save_block(int index, QString raw_text)
{
    save_block_range(index, 1, raw_text);
}

void NotesBridge::save_block_range(int start_index, int count, QString raw_text)
{
    if (start_index < 0 || count < 0) return;
    if (!ensureInitBlocking()) return;

    const QString pagePath = currentPageRelativePath();
    if (pagePath.isEmpty()) return;

    int rc = notes_core_page_save_block(
        rawConn(), qstrToFFI(m_notesPath), qstrToFFI(pagePath),
        start_index, count, qstrToFFI(raw_text),
        m_dropComments ? 1 : 0);

    if (rc < 0) {
        reportError(QStringLiteral("Failed to save block"));
        return;
    }

    /* Optimistic update: read back, parse, update UI immediately */
    char *source = notes_core_page_get_source(
        qstrToFFI(m_notesPath), qstrToFFI(pagePath));
    QString content = ffiStringToQString(source);
    if (!content.isEmpty()) {
        char *blocks = notes_core_page_parse_and_render_blocks_json(
            content.toUtf8().constData(),
            qstrToFFI(m_notesPath),
            m_dropComments ? 1 : 0,
            m_themeColorsJson.isEmpty() ? nullptr
                                        : qstrToFFI(m_themeColorsJson),
            qstrToFFI(buildOptionsJson()));
        m_currentBlocks = jsonArrayToQStringVariantList(
            ffiStringToQString(blocks));
        if (m_blockListModel) {
            m_blockListModel->setBlocks(m_currentBlocks);
        }
        m_blocksVersion++;
        emit page_changed();
    }

    /* Background reload for full refresh */
    load_page(pagePath);
}

void NotesBridge::append_to_current_page(QString text, bool is_task)
{
    ensureInit();
    const QString trimmed = text.trimmed();
    if (trimmed.isEmpty()) return;

    const bool isJournal = m_isJournalPage
        || m_currentPageName.compare(QLatin1String("Journal"),
                                     Qt::CaseInsensitive) == 0;
    if (isJournal) {
        int rc = notes_core_journal_append(
            rawConn(), qstrToFFI(m_notesPath),
            qstrToFFI(trimmed), is_task ? 1 : 0);
        if (rc < 0) {
            reportError(QStringLiteral("Failed to append to journal"));
            return;
        }
        load_page(QStringLiteral("journal.adoc"));
        return;
    }

    /* Regular page: read, append, save */
    const QString fullP = currentPageRelativePath();
    char *src = notes_core_page_get_source(
        qstrToFFI(m_notesPath), qstrToFFI(fullP));
    QString content = ffiStringToQString(src);

    if (!content.isEmpty() && !content.endsWith(QLatin1Char('\n')))
        content += QLatin1Char('\n');
    content += trimmed;
    content += QLatin1Char('\n');

    int rc = notes_core_page_save_source(
        rawConn(), qstrToFFI(m_notesPath), qstrToFFI(fullP),
        qstrToFFI(content));
    if (rc < 0) {
        reportError(QStringLiteral("Failed to append to note"));
        return;
    }
    load_page(fullP);
}

void NotesBridge::save_journal_block(int index, QString raw_text)
{
    if (index < 0) return;
    if (!ensureInitBlocking()) return;

    int rc = notes_core_page_save_block(
        rawConn(), qstrToFFI(m_notesPath), "journal.adoc",
        index, 1, qstrToFFI(raw_text), m_dropComments ? 1 : 0);

    if (rc < 0) {
        reportError(QStringLiteral("Failed to save journal block"));
        return;
    }
    loadMainPageDataSync();
    rebuildTreeInBackground();
}

void NotesBridge::toggle_journal_checkbox(int block_index, QString item_path)
{
    if (block_index < 0) return;
    if (!ensureInitBlocking()) return;

    int rc = notes_core_page_toggle_checkbox(
        rawConn(), qstrToFFI(m_notesPath), "journal.adoc",
        block_index, qstrToFFI(item_path));

    if (rc < 0) {
        reportError(QStringLiteral("Failed to toggle journal checkbox"));
        return;
    }
    loadMainPageDataSync();
    rebuildTreeInBackground();
}

void NotesBridge::append_to_journal(QString text, bool is_task)
{
    ensureInit();
    const QString trimmed = text.trimmed();
    if (trimmed.isEmpty()) return;

    int rc = notes_core_journal_append(
        rawConn(), qstrToFFI(m_notesPath),
        qstrToFFI(trimmed), is_task ? 1 : 0);
    if (rc < 0) {
        reportError(QStringLiteral("Failed to append to journal"));
        return;
    }
    loadMainPageDataSync();
    rebuildTreeInBackground();
}

QString NotesBridge::get_page_source(QString name)
{
    if (!ensureInitBlocking()) return QString();
    const QString fullPath = resolvePagePath(name);
    char *src = notes_core_page_get_source(
        qstrToFFI(m_notesPath), qstrToFFI(fullPath));
    return ffiStringToQString(src);
}

void NotesBridge::save_page_source(QString name, QString content)
{
    if (!ensureInitBlocking()) return;
    const QString fullPath = resolvePagePath(name);

    int rc = notes_core_page_save_source(
        rawConn(), qstrToFFI(m_notesPath), qstrToFFI(fullPath),
        qstrToFFI(content));
    if (rc < 0) {
        reportError(QStringLiteral("Failed to save page source"));
        return;
    }
    load_page(fullPath);
}

void NotesBridge::create_page(QString name)
{
    if (!ensureInitBlocking()) return;

    int rc = notes_core_page_create(
        rawConn(), qstrToFFI(m_notesPath), qstrToFFI(name));
    if (rc < 0) {
        reportError(QStringLiteral("Failed to create page: %1").arg(name));
        return;
    }
    rebuildTreeInBackground();

    /* Navigate to the new page */
    QString fullPath = name;
    if (!fullPath.endsWith(QLatin1String(".adoc")))
        fullPath += QLatin1String(".adoc");
    load_page(fullPath);
}

void NotesBridge::delete_page(QString name)
{
    if (!ensureInitBlocking()) return;

    /* Determine if the page being deleted is the currently-viewed one */
    const QString fullPath = resolvePagePath(name);
    const bool isCurrent =
        (m_currentPageName == name)
        || (m_currentPageFullPath == name)
        || (m_currentPageFullPath == fullPath);

    int rc = notes_core_page_delete(
        rawConn(), qstrToFFI(m_notesPath), qstrToFFI(name));
    if (rc < 0) {
        reportError(QStringLiteral("Failed to delete page: %1").arg(name));
        return;
    }

    if (isCurrent) {
        m_currentPageName.clear();
        m_currentPageGroupPath.clear();
        emit current_page_group_path_changed();
        m_currentPageFullPath.clear();
        emit current_page_full_path_changed();
        m_currentBlocks.clear();
        if (m_blockListModel) {
            m_blockListModel->clear();
        }
        m_isJournalPage = false;
        m_currentPageFilePath.clear();
        emit page_changed();
    }
    rebuildTreeInBackground();
}

void NotesBridge::navigate_to_page(QString name)
{
    load_page(name);
}

void NotesBridge::insert_link_at_cursor(int block_idx, int /*cursor_pos*/,
                                        QString target)
{
    if (block_idx < 0) return;
    if (!ensureInitBlocking()) return;

    const QString pagePath = currentPageRelativePath();
    if (pagePath.isEmpty()) return;

    /* Read current source */
    char *src = notes_core_page_get_source(
        qstrToFFI(m_notesPath), qstrToFFI(pagePath));
    QString content = ffiStringToQString(src);
    if (content.isEmpty()) return;

    /* Parse blocks to find the target block */
    char *blocksJson = notes_core_parse_blocks_json(
        qstrToFFI(content), m_dropComments ? 1 : 0);
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
        rawConn(), qstrToFFI(m_notesPath), qstrToFFI(pagePath),
        block_idx, 1, qstrToFFI(newRaw), m_dropComments ? 1 : 0);

    if (rc >= 0)
        load_page(pagePath);
}

void NotesBridge::toggle_checkbox(int block_index, QString item_path)
{
    if (block_index < 0) return;
    if (!ensureInitBlocking()) return;

    const QString pagePath = currentPageRelativePath();
    if (pagePath.isEmpty()) return;

    int rc = notes_core_page_toggle_checkbox(
        rawConn(), qstrToFFI(m_notesPath), qstrToFFI(pagePath),
        block_index, qstrToFFI(item_path));

    if (rc < 0) {
        reportError(QStringLiteral("Failed to toggle checkbox"));
        return;
    }
    if (m_blockListModel) {
        m_blockListModel->toggleCheckbox(block_index, item_path);
    }
    load_page(pagePath);
}

/* ================================================================== */
/*  Group operations                                                   */
/* ================================================================== */

bool NotesBridge::create_group(QString parent_path, QString name)
{
    if (!ensureInitBlocking()) return false;

    int rc = notes_core_group_create(
        rawConn(), qstrToFFI(m_notesPath),
        qstrToFFI(parent_path), qstrToFFI(name));
    if (rc < 0) {
        reportError(QStringLiteral("Failed to create group: %1").arg(name));
        return false;
    }
    rebuildTreeInBackground();
    return true;
}

bool NotesBridge::rename_group(QString old_path, QString new_name)
{
    if (!ensureInitBlocking()) return false;

    int rc = notes_core_group_rename(
        rawConn(), qstrToFFI(m_notesPath),
        qstrToFFI(old_path), qstrToFFI(new_name));
    if (rc < 0) {
        reportError(QStringLiteral("Failed to rename group"));
        return false;
    }
    rebuildTreeInBackground();
    return true;
}

bool NotesBridge::delete_group(QString path, bool recursive)
{
    if (!ensureInitBlocking()) return false;

    int rc = notes_core_group_delete(
        rawConn(), qstrToFFI(m_notesPath),
        qstrToFFI(path), recursive ? 1 : 0);
    if (rc < 0) {
        reportError(QStringLiteral("Failed to delete group"));
        return false;
    }
    rebuildTreeInBackground();
    return true;
}

bool NotesBridge::move_page_to_group(QString page_full_path,
                                     QString target_group)
{
    if (!ensureInitBlocking()) return false;

    int rc = notes_core_page_move(
        rawConn(), qstrToFFI(m_notesPath),
        qstrToFFI(page_full_path), qstrToFFI(target_group));
    if (rc < 0) {
        reportError(QStringLiteral("Failed to move page"));
        return false;
    }

    /* Update current page properties if moved page is the active one */
    if (m_currentPageFullPath == page_full_path) {
        int sl = page_full_path.lastIndexOf(QLatin1Char('/'));
        QString filename = (sl >= 0) ? page_full_path.mid(sl + 1)
                                     : page_full_path;
        m_currentPageGroupPath = target_group;
        emit current_page_group_path_changed();
        m_currentPageFullPath = target_group.isEmpty()
            ? filename
            : target_group + QLatin1Char('/') + filename;
        emit current_page_full_path_changed();
        m_currentPageFilePath = m_currentPageFullPath;
        emit page_changed();
    }

    rebuildTreeInBackground();
    return true;
}

void NotesBridge::set_group_display_depth(int depth)
{
    if (m_groupDisplayDepth != depth) {
        m_groupDisplayDepth = depth;
        emit group_depth_changed();
        rebuildTreeInBackground();
    }
}

bool NotesBridge::toggle_group_collapsed(QString group_path)
{
    if (!ensureInitBlocking()) return false;

    int rc = notes_core_group_toggle_collapsed(
        rawConn(), qstrToFFI(group_path));

    bool collapsed = (rc >= 0);
    if (!collapsed) {
        reportError(QStringLiteral("Failed to toggle group collapsed state"));
        return false;
    }

    /* Rebuild tree in background (DB toggle is already persisted) */
    rebuildTreeInBackground();
    return collapsed;
}

bool NotesBridge::set_group_note_sort(QString group_path, QString note_sort)
{
    if (!ensureInitBlocking()) return false;

    /* Convert sort string to int: "newest"->0, "oldest"->1, "alphabetical"->2 */
    int sortOrder = 0;
    if (note_sort == QLatin1String("oldest"))
        sortOrder = 1;
    else if (note_sort == QLatin1String("alphabetical"))
        sortOrder = 2;

    int rc = notes_core_group_set_note_sort(
        rawConn(), qstrToFFI(group_path), sortOrder);
    if (rc < 0) {
        reportError(QStringLiteral("Failed to set group note sort"));
        return false;
    }
    rebuildTreeInBackground();
    return true;
}

QString NotesBridge::get_group_note_sort(QString group_path)
{
    if (!ensureInitBlocking()) return QStringLiteral("newest");

    int sort = notes_core_group_get_note_sort(
        rawConn(), qstrToFFI(group_path));

    switch (sort) {
    case 1:  return QStringLiteral("oldest");
    case 2:  return QStringLiteral("alphabetical");
    default: return QStringLiteral("newest");
    }
}

QString NotesBridge::get_groups_json()
{
    if (!ensureInitBlocking()) return QStringLiteral("[]");

    char *json = notes_core_groups_flat_json(rawConn());
    return ffiStringToQString(json);
}

/* ================================================================== */
/*  Index                                                              */
/* ================================================================== */

QString NotesBridge::rebuild_index()
{
    if (!ensureInitBlocking())
        return QStringLiteral("Error: database not ready");

    int rc = notes_core_rebuild_index(rawConn(), qstrToFFI(m_notesPath));
    if (rc < 0) {
        reportError(QStringLiteral("Failed to rebuild index"));
        return QStringLiteral("Error: rebuild failed");
    }
    rebuildTreeInBackground();
    return QStringLiteral("Index rebuilt");
}

/* ================================================================== */
/*  Search                                                             */
/* ================================================================== */

void NotesBridge::do_search(QString query)
{
    ensureInit();
    m_searchQuery = query;

    if (query.trimmed().isEmpty()) {
        m_searchResults.clear();
        m_searchLoading = false;
        emit search_results_changed();
        emit loading_changed();
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
    emit loading_changed();

    const int gen = ++m_searchGeneration;
    const std::string dbP = dbPathFor(m_dataDir).toStdString();
    const std::string q   = query.toStdString();

    std::thread([this, dbP, q, gen]() {
        /* Create a fresh search engine for this query */
        FfiSearchEngine *engine = notes_core_search_new();
        int rc = notes_core_search_start(engine, dbP.c_str(), q.c_str());

        if (rc < 0) {
            notes_core_search_free(engine);
            if (gen != m_searchGeneration.load()) return;
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

            {
                std::lock_guard<std::mutex> lk(m_pendingSearchMutex);
                m_pendingSearchHits = std::move(hits);
            }

            QMetaObject::invokeMethod(this, [this]() {
                this->poll_search();
            }, Qt::QueuedConnection);

            /* Kick off background preview generation */
            std::thread([this, gen]() {
                if (gen != m_searchGeneration.load()) return;

                QList<SearchHit> snapshot;
                {
                    std::lock_guard<std::mutex> lk(m_pendingSearchMutex);
                    if (!m_pendingSearchHits.has_value()) return;
                    snapshot = *m_pendingSearchHits;
                }

                const std::string nd = m_notesPath.toStdString();
                const bool drop = m_dropComments;

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

                QMetaObject::invokeMethod(this, [this]() {
                    this->poll_search_previews();
                }, Qt::QueuedConnection);
            }).detach();

        } else {
            std::lock_guard<std::mutex> lk(m_pendingSearchMutex);
            m_pendingSearchError = QStringLiteral("Search failed");
            m_pendingSearchHits  = QList<SearchHit>();

            QMetaObject::invokeMethod(this, [this]() {
                this->poll_search();
            }, Qt::QueuedConnection);
        }

        notes_core_search_free(engine);
    }).detach();
}

void NotesBridge::search(QString query)
{
    m_searchQuery = query;
    m_searchResults.clear();
    m_searchLoading = false;
    emit search_results_changed();
    emit loading_changed();
}

bool NotesBridge::poll_search()
{
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

    if (!hasResult)
        return false;

    m_searchLoading = false;
    emit loading_changed();

    if (!error.isEmpty()) {
        reportError(error);
        return true;
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
    emit search_results_changed();
    return true;
}

bool NotesBridge::poll_search_previews()
{
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

    if (!hasPreviews)
        return false;

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
        emit search_results_changed();
    }
    return true;
}

QString NotesBridge::get_linkable_pages_json(QString query)
{
    if (!ensureInitBlocking()) return QStringLiteral("[]");

    char *json = notes_core_recent_pages_json(rawConn(), 1000);
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

/* ================================================================== */
/*  Main page data (journal + tree + recent pages)                     */
/* ================================================================== */

void NotesBridge::loadMainPageDataSync()
{
    /* Journal data — fast synchronous FFI calls */
    char *lines = notes_core_journal_recent_lines(
        qstrToFFI(m_notesPath), 5);
    QString linesStr = ffiStringToQString(lines);

    m_recentJournalLines.clear();
    m_journalBlocks.clear();

    QJsonDocument jdoc = QJsonDocument::fromJson(linesStr.toUtf8());
    if (jdoc.isArray()) {
        for (const QJsonValue &v : jdoc.array())
            m_recentJournalLines.append(v.toString());
    }

    /* Get recent pages for the initial quick display */
    if (rawConn()) {
        char *recent = notes_core_recent_pages_json(rawConn(), 10);
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

    emit data_refreshed();
}

void NotesBridge::load_main_page_data()
{
    ensureInit();
    if (!pollInit()) return;

    loadMainPageDataSync();
    rebuildTreeInBackground();
}

bool NotesBridge::poll_main_page_data()
{
    /* If DB init was pending, poll it and trigger data load upon completion */
    if (!conn_) {
        if (pollInit())
            load_main_page_data();
        return false;
    }

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

    if (!hasResult)
        return false;

    if (!data.recentPageJsons.isEmpty()) {
        QVariantList list;
        for (const QString &j : data.recentPageJsons)
            list.append(j);
        m_recentPages = list;
    }
    m_groupedTreeJson = data.groupedTreeJson;
    emit data_refreshed();
    return true;
}

bool NotesBridge::poll_results()
{
    PendingPageResult result;
    bool hasResult = false;

    {
        std::lock_guard<std::mutex> lk(m_pendingMutex);
        if (m_pendingPage.has_value()) {
            result = std::move(*m_pendingPage);
            m_pendingPage.reset();
            hasResult = true;
        }
    }

    if (!hasResult)
        return false;

    m_isLoading = false;
    emit loading_changed();

    if (!result.error.isEmpty()) {
        reportError(result.error);
        return true;
    }

    m_currentBlocks = result.blocks;
    if (m_blockListModel) {
        m_blockListModel->setBlocks(m_currentBlocks);
    }
    m_blocksVersion++;
    emit page_changed();
    return true;
}

/* ================================================================== */
/*  Export                                                             */
/* ================================================================== */

QString NotesBridge::export_html(QString page_name)
{
    ensureInit();

    QString fullPath;
    if (m_isJournalPage
        || page_name.compare(QLatin1String("Journal"), Qt::CaseInsensitive) == 0)
        fullPath = QStringLiteral("journal.adoc");
    else
        fullPath = resolvePagePath(page_name);

    const QString home = QString::fromLocal8Bit(qgetenv("HOME"));
    if (home.isEmpty()) return QString();
    const QString exportDir = home
        + QStringLiteral("/Documents/Notes++ Exports");
    QDir().mkpath(exportDir);

    const QString title = extractTitle(page_name);
    const QString outputPath = exportDir + QLatin1Char('/') + title
        + QLatin1String(".html");
    const QString absPath = m_notesPath + QLatin1Char('/') + fullPath;

    char *result = notes_core_export_html5(
        qstrToFFI(m_notesPath), qstrToFFI(fullPath),
        qstrToFFI(absPath), qstrToFFI(outputPath));

    QString path = ffiStringToQString(result);
    if (path.isEmpty())
        reportError(QStringLiteral("Export failed"));
    return path;
}

QString NotesBridge::export_all_html()
{
    ensureInit();

    const QString home = QString::fromLocal8Bit(qgetenv("HOME"));
    if (home.isEmpty()) return QString();
    const QString exportDir = home
        + QStringLiteral("/Documents/Notes++ Exports");
    QDir().mkpath(exportDir);

    char *result = notes_core_export_all_html5(
        qstrToFFI(m_notesPath), qstrToFFI(exportDir));
    return ffiStringToQString(result);
}

QString NotesBridge::open_in_browser(QString page_name)
{
    if (m_webServerRunning && server_) {
        uint16_t port = notes_core_server_port(server_.get());
        char *urlsJson = notes_core_server_urls_json(server_.get());
        QString urls = ffiStringToQString(urlsJson);

        QJsonDocument doc = QJsonDocument::fromJson(urls.toUtf8());
        QJsonArray arr = doc.array();
        QString base;
        if (!arr.isEmpty()) {
            base = arr[0].toString();
            base.replace(QLatin1String("0.0.0.0"),
                         QLatin1String("localhost"));
        } else {
            base = QStringLiteral("http://localhost:%1").arg(port);
        }

        QString fullPath;
        if (m_isJournalPage
            || page_name.compare(QLatin1String("Journal"),
                                 Qt::CaseInsensitive) == 0)
            fullPath = QStringLiteral("journal.adoc");
        else
            fullPath = resolvePagePath(page_name);

        return base + QStringLiteral("/page/") + fullPath;
    }
    return export_html(page_name);
}

/* ================================================================== */
/*  Server                                                             */
/* ================================================================== */

QString NotesBridge::get_server_urls_json()
{
    if (!server_) return QStringLiteral("[]");
    char *json = notes_core_server_urls_json(server_.get());
    return ffiStringToQString(json);
}

QString NotesBridge::start_web_server()
{
    ensureInit();
    if (m_webServerRunning)
        return m_webServerUrl;

    const std::string notesDir = m_notesPath.toStdString();
    const std::string dbP      = dbPathFor(m_dataDir).toStdString();
    const std::string backupDir =
        (m_dataDir + QStringLiteral("/backups")).toStdString();
    const std::string certDir =
        (m_dataDir + QStringLiteral("/tls")).toStdString();

    /* Build config JSON for the FFI server start */
    QJsonObject config;
    config[QStringLiteral("bind_address")]          = m_bindAddress;
    config[QStringLiteral("reject_public_networks")] = m_rejectPublicNetworks;
    config[QStringLiteral("enable_tls")]             = true;
    config[QStringLiteral("tls_cert_path")] =
        QString::fromStdString(certDir) + QStringLiteral("/server.crt");
    config[QStringLiteral("tls_key_path")] =
        QString::fromStdString(certDir) + QStringLiteral("/server.key");

    QJsonObject llm;
    llm[QStringLiteral("provider")]        = m_llmProvider;
    llm[QStringLiteral("endpoint_url")]    = m_llmEndpointUrl;
    llm[QStringLiteral("model")]           = m_llmModel;
    llm[QStringLiteral("api_key")]         = m_llmApiKey;
    llm[QStringLiteral("timeout_secs")]    = m_llmTimeoutSecs;
    llm[QStringLiteral("allow_self_signed")] = m_allowSelfSigned;
    config[QStringLiteral("llm")]          = llm;

    QJsonObject perms;
    perms[QStringLiteral("auto_allow_read")]      = m_autoAllowRead;
    perms[QStringLiteral("auto_allow_create")]    = m_autoAllowCreate;
    perms[QStringLiteral("require_confirm_edit")] = m_requireConfirmEdit;
    perms[QStringLiteral("allow_fetch_url")]      = m_allowFetchUrl;
    config[QStringLiteral("permissions")]         = perms;

    QJsonObject auth;
    auth[QStringLiteral("session_expiry_secs")]   = m_sessionExpirySecs;
    config[QStringLiteral("auth")]                = auth;

    const std::string configStr = QString::fromUtf8(
        QJsonDocument(config).toJson(QJsonDocument::Compact)).toStdString();

    HttpServerHandle *handle = notes_core_server_start(
        notesDir.c_str(), dbP.c_str(), backupDir.c_str(),
        8080, configStr.c_str());

    if (!handle) {
        reportError(QStringLiteral("Failed to start web server"));
        return QString();
    }

    server_.reset(handle);

    char *urlsJson = notes_core_server_urls_json(handle);
    QJsonDocument urlDoc = QJsonDocument::fromJson(
        ffiStringToQString(urlsJson).toUtf8());
    QJsonArray urls = urlDoc.array();

    QString primaryUrl;
    if (!urls.isEmpty()) {
        primaryUrl = urls[0].toString();
        primaryUrl.replace(QLatin1String("0.0.0.0"),
                           QLatin1String("localhost"));
    } else {
        uint16_t port = notes_core_server_port(handle);
        primaryUrl = QStringLiteral("http://localhost:%1").arg(port);
    }

    m_webServerUrl     = primaryUrl;
    m_webServerRunning = true;
    emit web_server_status_changed();
    return primaryUrl;
}

void NotesBridge::stop_web_server()
{
    if (server_)
        server_.reset();
    m_webServerRunning = false;
    m_webServerUrl.clear();
    emit web_server_status_changed();
}

bool NotesBridge::toggle_web_server()
{
    if (m_webServerRunning) {
        stop_web_server();
        return false;
    }
    return !start_web_server().isEmpty();
}

void NotesBridge::configure_ai(QString provider, QString url, QString model,
                               QString key, int timeout, bool auto_read,
                               bool auto_create, bool require_edit,
                               bool allow_self_signed, bool allow_fetch)
{
    m_llmProvider      = provider;
    m_llmEndpointUrl   = url.trimmed();
    m_llmModel         = model.trimmed();
    m_llmApiKey        = key.trimmed();
    m_llmTimeoutSecs   = (timeout > 0) ? timeout : 90;
    m_autoAllowRead    = auto_read;
    m_autoAllowCreate  = auto_create;
    m_requireConfirmEdit = require_edit;
    m_allowSelfSigned  = allow_self_signed;
    m_allowFetchUrl    = allow_fetch;
}

/* ================================================================== */
/*  TLS                                                                */
/* ================================================================== */

QString NotesBridge::install_tls_certificate(QString cert_pem_or_path,
                                             QString key_pem_or_path)
{
    ensureInit();

    const QString certPath = m_dataDir + QStringLiteral("/tls/server.crt");
    const QString keyPath  = m_dataDir + QStringLiteral("/tls/server.key");

    /* Ensure TLS directory exists */
    QDir().mkpath(m_dataDir + QStringLiteral("/tls"));

    char *err = notes_core_server_tls_install(
        qstrToFFI(certPath), qstrToFFI(keyPath),
        qstrToFFI(cert_pem_or_path), qstrToFFI(key_pem_or_path));

    QString error = ffiStringToQString(err);
    if (!error.isEmpty()) {
        reportError(QStringLiteral("Failed to install SSL certificate: ")
                    + error);
        return error;
    }

    /* Restart server if running to pick up new cert */
    if (m_webServerRunning) {
        stop_web_server();
        start_web_server();
    }
    return QString();
}

QString NotesBridge::reset_tls_certificate()
{
    ensureInit();

    const QString certPath = m_dataDir + QStringLiteral("/tls/server.crt");
    const QString keyPath  = m_dataDir + QStringLiteral("/tls/server.key");

    char *err = notes_core_server_tls_reset(
        qstrToFFI(certPath), qstrToFFI(keyPath));

    QString error = ffiStringToQString(err);
    if (!error.isEmpty()) {
        reportError(QStringLiteral("Failed to reset SSL certificate: ")
                    + error);
        return error;
    }

    if (m_webServerRunning) {
        stop_web_server();
        start_web_server();
    }
    return QString();
}

bool NotesBridge::is_custom_tls_certificate()
{
    const QString certPath = m_dataDir + QStringLiteral("/tls/server.crt");
    return notes_core_server_tls_is_custom(qstrToFFI(certPath)) != 0;
}

QString NotesBridge::get_tls_certificate_info_json()
{
    const QString certPath = m_dataDir + QStringLiteral("/tls/server.crt");
    const QString keyPath  = m_dataDir + QStringLiteral("/tls/server.key");
    const bool isCustom =
        notes_core_server_tls_is_custom(qstrToFFI(certPath)) != 0;

    QJsonObject info;
    info[QStringLiteral("is_custom")] = isCustom;
    info[QStringLiteral("cert_path")] = certPath;
    info[QStringLiteral("key_path")]  = keyPath;
    info[QStringLiteral("exists")]    =
        QFile::exists(certPath) && QFile::exists(keyPath);

    return QString::fromUtf8(
        QJsonDocument(info).toJson(QJsonDocument::Compact));
}

/* ================================================================== */
/*  Settings                                                           */
/* ================================================================== */

void NotesBridge::set_drop_comments(bool drop)
{
    if (m_dropComments != drop) {
        m_dropComments = drop;
        emit drop_comments_changed();
        if (!m_currentPageName.isEmpty())
            load_page(m_currentPageName);
        rebuildTreeInBackground();
    }
}

void NotesBridge::set_reject_public_networks(bool reject)
{
    if (m_rejectPublicNetworks != reject) {
        m_rejectPublicNetworks = reject;
        emit reject_public_networks_changed();
    }
}

void NotesBridge::set_bind_address(QString addr)
{
    if (m_bindAddress != addr) {
        m_bindAddress = addr;
        emit bind_address_changed();
    }
}

QString NotesBridge::get_network_interfaces_json()
{
    char *json = notes_core_get_network_interfaces_json();
    return ffiStringToQString(json);
}

void NotesBridge::set_theme(QString colors_json)
{
    m_themeColorsJson = colors_json;
}

void NotesBridge::set_session_expiry_hours(int hours)
{
    m_sessionExpirySecs = (hours > 0 ? hours : 1) * 3600;
}

/* ================================================================== */
/*  Auth                                                               */
/* ================================================================== */

bool NotesBridge::check_auth_challenge()
{
    /* Full implementation requires server context access.
     * Basic version returns current pending state. */
    return m_authChallengePending;
}

void NotesBridge::approve_auth_challenge(QString challenge_id)
{
    Q_UNUSED(challenge_id);
    m_authChallengePending = false;
    m_authChallengeId.clear();
    m_authVerificationCode.clear();
    emit auth_challenge_changed();
}

void NotesBridge::deny_auth_challenge(QString challenge_id)
{
    Q_UNUSED(challenge_id);
    m_authChallengePending = false;
    m_authChallengeId.clear();
    m_authVerificationCode.clear();
    emit auth_challenge_changed();
}

/* ================================================================== */
/*  Rendering                                                          */
/* ================================================================== */

QString NotesBridge::render_element_previews()
{
    /* Hardcoded AsciiDoc snippets — identical to Rust bridge */
    static const char *kSnippets[] = {
        "==== Section Title",
        "This is *bold* text",
        "This is _italic_ text",
        "Use `printf()` here",
        "This has ~deleted~ text",
        "E = mc^2^",
        "This is #highlighted# text",
        "Referencefootnote:[An important note.] here",
        "===== Deep Title",
        "====== Deepest Title",
        "[source]\n----\nfn main() {\n    println!(\"hello\");\n}\n----",
        "[quote]\n____\nFamous words.\n____",
        "[verse]\n____\nThe road goes ever on.\n____",
        "....\n  Literal text here\n....",
        ".Example\n====\nExample content\n====",
        "--\nOpen block content\n--",
        "---",
        "<<<",
        "[NOTE]\n====\nNote text.\n====",
        "[TIP]\n====\nTip text.\n====",
        "[WARNING]\n====\nWarning text.\n====",
        "[CAUTION]\n====\nBe very careful.\n====",
        "[IMPORTANT]\n====\nThis is critical.\n====",
        "Term:: Description text",
        "|===\n| Name | Age\n| Alice | 30\n|===",
        "image::photo.jpg[A photo]",
        "See image:icon.png[Icon,16] here",
        "[sidebar]\n****\nSidebar text.\n****",
        "Click icon:star[] to rate",
        "Press kbd:[Ctrl+S] to save",
        "Click btn:[Submit] to continue",
        "Use menu:File[Quit] to exit",
        "See xref:other.adoc[Other Page]",
        "The equation stem:[E = mc^2]",
        "Use pass:[<b>raw HTML</b>] here",
        "A ((concept)) in text",
        "////\nThis is a block comment\n////",
        "// This is a line comment",
        ":author: Jane Doe",
        nullptr
    };

    const std::string opts  = buildOptionsJson().toStdString();
    const std::string theme = m_themeColorsJson.toStdString();
    const int drop = m_dropComments ? 1 : 0;

    QJsonObject map;
    for (int i = 0; kSnippets[i] != nullptr; ++i) {
        const char *snippet = kSnippets[i];

        /* Parse to get block list */
        char *blocksJson = notes_core_parse_blocks_json(snippet, drop);
        QString blocksStr = ffiStringToQString(blocksJson);
        QJsonDocument doc = QJsonDocument::fromJson(blocksStr.toUtf8());
        QJsonArray arr = doc.array();

        QString html;
        if (!arr.isEmpty()) {
            /* Render the first block */
            QString blockJson = QString::fromUtf8(
                QJsonDocument(arr[0].toObject())
                    .toJson(QJsonDocument::Compact));
            char *rendered = notes_core_render_qt_block_json(
                qstrToFFI(blockJson), 0,
                theme.empty() ? nullptr : theme.c_str(),
                opts.empty()  ? nullptr : opts.c_str());
            html = ffiStringToQString(rendered);
        }

        map[QString::fromUtf8(snippet)] = html;
    }

    return QString::fromUtf8(
        QJsonDocument(map).toJson(QJsonDocument::Compact));
}
