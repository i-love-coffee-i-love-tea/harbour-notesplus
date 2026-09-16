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
#include <QStandardPaths>
#include <cstdlib>

/* ================================================================== */
/*  Static helpers                                                     */
/* ================================================================== */

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
    , m_alive(std::make_shared<std::atomic<bool>>(true))
{
    m_notesPath = ffiStringToQString(
        notes_core_app_paths_notes_dir(paths_.get()));
    m_dataDir = ffiStringToQString(
        notes_core_app_paths_data_dir(paths_.get()));

    /* Default property values (mirror Rust Default::default()) */
    m_notesDir        = m_notesPath;
    m_groupedTreeJson = QStringLiteral("[]");
    m_bindAddress     = QStringLiteral("0.0.0.0");

    /* Construct BridgeContext for domain classes. */
    BridgeContext ctx{
        [this]() -> void* { return conn_.get(); },
        m_notesPath,
        m_dataDir,
        m_alive,
        [this]() -> bool { return m_dropComments; },
        [this]() -> QString { return m_themeColorsJson; },
        [this](const QString &msg) { reportError(msg); },
        [this]() -> QString { return buildOptionsJson(); }
    };

    /* Domain classes */
    m_pageStore = std::make_unique<PageStore>(ctx, m_blockListModel);
    m_pageStore->setRebuildTreeCallback([this]() {
        if (m_mainPageLoader) m_mainPageLoader->rebuildTreeInBackground(this);
    });

    m_groupManager = std::make_unique<GroupManager>(ctx);
    m_groupManager->setRebuildTreeCallback([this]() {
        if (m_mainPageLoader) m_mainPageLoader->rebuildTreeInBackground(this);
    });

    m_mainPageLoader = std::make_unique<MainPageLoader>(ctx);

    m_searchManager = std::make_unique<SearchManager>(ctx);

    m_serverManager = std::make_unique<ServerManager>(ctx);
    m_serverManager->setOnPageReloadNeeded([this]() {
        if (!m_currentPageName.isEmpty())
            m_pageStore->load_page(m_currentPageName, this);
    });
    m_serverManager->setOnTreeRebuildNeeded([this]() {
        if (m_mainPageLoader) m_mainPageLoader->rebuildTreeInBackground(this);
    });

    m_renderHelper = std::make_unique<RenderHelper>(ctx);

    m_exportHelper = std::make_unique<ExportHelper>(
        ctx,
        [this]() -> bool { return m_webServerRunning; },
        [this]() -> QString { return m_webServerUrl; },
        [this](const QString &name) -> QString { return resolvePagePath(name); }
    );

    /* Kick off background DB init */
    ensureInit();
}

NotesBridge::~NotesBridge()
{
    m_alive->store(false, std::memory_order_release);
}

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

    auto alive = m_alive;
    QtConcurrent::run([this, alive, dataDir, notesDir, dbPath]() {
        /* Create directories */
        QDir().mkpath(QString::fromStdString(notesDir));
        QDir().mkpath(QString::fromStdString(dataDir) + QStringLiteral("/exports"));

        /* Open database */
        void *raw = notes_core_db_open(dbPath.c_str());
        if (raw) {
            /* Rebuild index so the DB knows about all .adoc files */
            notes_core_rebuild_index(raw, notesDir.c_str());
        }

        if (!alive->load(std::memory_order_acquire)) return;
        {
            std::lock_guard<std::mutex> lk(m_initMutex);
            m_initResult = raw;
            m_initDone   = true;
        }
        m_initCv.notify_one();

        QMetaObject::invokeMethod(this, "poll_init_and_load", Qt::QueuedConnection);
    });
}

void NotesBridge::poll_init_and_load()
{
    if (pollInit()) {
        load_main_page_data();
    }
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
    return m_pageStore->resolvePagePath(name);
}

QString NotesBridge::currentPageRelativePath()
{
    return m_pageStore->currentPageRelativePath();
}

/* ================================================================== */
/*  Background tree rebuild (delegated to MainPageLoader)              */
/* ================================================================== */

void NotesBridge::rebuildTreeInBackground()
{
    m_mainPageLoader->rebuildTreeInBackground(this);
}

/* ================================================================== */
/*  Page operations (delegated to PageStore)                           */
/* ================================================================== */

void NotesBridge::load_page(QString name)
{
    if (!ensureInitBlocking()) return;
    m_pageStore->load_page(name, this);
    m_isLoading = m_pageStore->isLoading();
    emit loading_changed();
    /* Sync facade properties from PageStore */
    m_currentPageName      = m_pageStore->currentPageName();
    m_currentPageGroupPath = m_pageStore->currentPageGroupPath();
    emit current_page_group_path_changed();
    m_currentPageFullPath  = m_pageStore->currentPageFullPath();
    emit current_page_full_path_changed();
    m_currentPageFilePath  = m_pageStore->currentPageFilePath();
    m_isJournalPage        = m_pageStore->isJournalPage();
    emit page_changed();
}

void NotesBridge::save_block(int index, QString raw_text)
{
    save_block_range(index, 1, raw_text);
}

void NotesBridge::save_block_range(int start_index, int count, QString raw_text)
{
    if (!ensureInitBlocking()) return;
    m_pageStore->save_block_range(start_index, count, raw_text, this);
    m_currentBlocks = m_pageStore->currentBlocks();
    m_blocksVersion = m_pageStore->blocksVersion();
    if (m_blockListModel) m_blockListModel->setBlocks(m_currentBlocks);
    emit page_changed();
}

void NotesBridge::append_to_current_page(QString text, bool is_task)
{
    ensureInit();
    m_pageStore->append_to_current_page(text, is_task, this);
}

void NotesBridge::save_journal_block(int index, QString raw_text)
{
    if (!ensureInitBlocking()) return;
    m_pageStore->save_journal_block(index, raw_text);
    m_mainPageLoader->loadMainPageDataSync();
    m_mainPageLoader->rebuildTreeInBackground(this);
}

void NotesBridge::toggle_journal_checkbox(int block_index, QString item_path)
{
    if (!ensureInitBlocking()) return;
    m_pageStore->toggle_journal_checkbox(block_index, item_path);
    m_mainPageLoader->loadMainPageDataSync();
    m_mainPageLoader->rebuildTreeInBackground(this);
}

void NotesBridge::append_to_journal(QString text, bool is_task)
{
    ensureInit();
    m_pageStore->append_to_journal(text, is_task);
    m_mainPageLoader->loadMainPageDataSync();
    m_mainPageLoader->rebuildTreeInBackground(this);
}

QString NotesBridge::get_page_source(QString name)
{
    if (!ensureInitBlocking()) return QString();
    return m_pageStore->get_page_source(name);
}

void NotesBridge::save_page_source(QString name, QString content)
{
    if (!ensureInitBlocking()) return;
    m_pageStore->save_page_source(name, content, this);
}

void NotesBridge::create_page(QString name)
{
    if (!ensureInitBlocking()) return;
    m_pageStore->create_page(name, this);
}

void NotesBridge::delete_page(QString name)
{
    if (!ensureInitBlocking()) return;
    m_pageStore->delete_page(name, this);
    /* Sync facade state after delete */
    m_currentPageName      = m_pageStore->currentPageName();
    m_currentPageGroupPath = m_pageStore->currentPageGroupPath();
    emit current_page_group_path_changed();
    m_currentPageFullPath  = m_pageStore->currentPageFullPath();
    emit current_page_full_path_changed();
    m_currentPageFilePath  = m_pageStore->currentPageFilePath();
    m_currentBlocks        = m_pageStore->currentBlocks();
    if (m_blockListModel) m_blockListModel->clear();
    m_isJournalPage        = m_pageStore->isJournalPage();
    emit page_changed();
}

bool NotesBridge::rename_page(QString old_path, QString new_title)
{
    if (!ensureInitBlocking()) return false;
    return m_pageStore->rename_page(old_path, new_title, this);
}

void NotesBridge::navigate_to_page(QString name)
{
    load_page(name);
}

void NotesBridge::insert_link_at_cursor(int block_idx, int cursor_pos,
                                        QString target)
{
    if (!ensureInitBlocking()) return;
    m_pageStore->insert_link_at_cursor(block_idx, cursor_pos, target, this);
}

void NotesBridge::toggle_checkbox(int block_index, QString item_path)
{
    if (!ensureInitBlocking()) return;
    m_pageStore->toggle_checkbox(block_index, item_path, this);
}

/* ================================================================== */
/*  Group operations (delegated to GroupManager)                       */
/* ================================================================== */

bool NotesBridge::create_group(QString parent_path, QString name)
{
    if (!ensureInitBlocking()) return false;
    return m_groupManager->create_group(parent_path, name);
}

bool NotesBridge::rename_group(QString old_path, QString new_name)
{
    if (!ensureInitBlocking()) return false;
    return m_groupManager->rename_group(old_path, new_name);
}

bool NotesBridge::delete_group(QString path, bool recursive)
{
    if (!ensureInitBlocking()) return false;
    return m_groupManager->delete_group(path, recursive);
}

bool NotesBridge::move_page_to_group(QString page_full_path,
                                     QString target_group)
{
    if (!ensureInitBlocking()) return false;
    auto result = m_groupManager->move_page_to_group(
        page_full_path, target_group, m_currentPageFullPath);
    if (!result.success) return false;

    if (result.currentPageMoved) {
        m_currentPageGroupPath = result.newGroupPath;
        emit current_page_group_path_changed();
        m_currentPageFullPath  = result.newFullPath;
        emit current_page_full_path_changed();
        m_currentPageFilePath  = result.newFilePath;
        emit page_changed();
    }
    return true;
}

void NotesBridge::set_group_display_depth(int depth)
{
    if (m_groupManager->set_group_display_depth(depth)) {
        m_groupDisplayDepth = depth;
        emit group_depth_changed();
        m_mainPageLoader->setGroupDisplayDepth(depth);
        m_mainPageLoader->rebuildTreeInBackground(this);
    }
}

bool NotesBridge::toggle_group_collapsed(QString group_path)
{
    if (!ensureInitBlocking()) return false;
    return m_groupManager->toggle_group_collapsed(group_path);
}

bool NotesBridge::set_group_note_sort(QString group_path, QString note_sort)
{
    if (!ensureInitBlocking()) return false;
    return m_groupManager->set_group_note_sort(group_path, note_sort);
}

QString NotesBridge::get_group_note_sort(QString group_path)
{
    if (!ensureInitBlocking()) return QStringLiteral("newest");
    return m_groupManager->get_group_note_sort(group_path);
}

QString NotesBridge::get_groups_json()
{
    if (!ensureInitBlocking()) return QStringLiteral("[]");
    return m_groupManager->get_groups_json();
}

/* ================================================================== */
/*  Index (delegated to PageStore)                                     */
/* ================================================================== */

QString NotesBridge::rebuild_index()
{
    if (!ensureInitBlocking())
        return QStringLiteral("Error: database not ready");
    return m_pageStore->rebuild_index();
}

/* ================================================================== */
/*  Search (delegated to SearchManager)                                */
/* ================================================================== */

void NotesBridge::do_search(QString query)
{
    ensureInit();
    m_searchQuery = query;
    m_searchManager->do_search(query, this);
    m_searchResults = m_searchManager->searchResults();
    m_searchLoading = m_searchManager->searchLoading();
    emit search_results_changed();
    emit loading_changed();
}

void NotesBridge::search(QString query)
{
    m_searchQuery = query;
    m_searchManager->search(query);
    m_searchResults.clear();
    m_searchLoading = false;
    emit search_results_changed();
    emit loading_changed();
}

bool NotesBridge::poll_search()
{
    auto result = m_searchManager->poll_search();
    if (!result.hasResult) return false;

    m_searchLoading = result.loading;
    emit loading_changed();

    if (!result.error.isEmpty()) {
        reportError(result.error);
        return true;
    }

    m_searchResults = result.results;
    emit search_results_changed();
    return true;
}

bool NotesBridge::poll_search_previews()
{
    auto result = m_searchManager->poll_search_previews();
    if (!result.hasResult) return false;

    if (!result.results.isEmpty()) {
        m_searchResults = result.results;
        emit search_results_changed();
    }
    return true;
}

QString NotesBridge::get_linkable_pages_json(QString query)
{
    if (!ensureInitBlocking()) return QStringLiteral("[]");
    return m_searchManager->get_linkable_pages_json(query);
}

/* ================================================================== */
/*  Main page data (delegated to MainPageLoader)                       */
/* ================================================================== */

void NotesBridge::loadMainPageDataSync()
{
    m_mainPageLoader->loadMainPageDataSync();
    m_recentPages          = m_mainPageLoader->recentPages();
    m_groupedTreeJson      = m_mainPageLoader->groupedTreeJson();
    m_recentJournalLines   = m_mainPageLoader->recentJournalLines();
    m_journalBlocks        = m_mainPageLoader->journalBlocks();
    emit data_refreshed();
}

void NotesBridge::load_main_page_data()
{
    ensureInit();
    if (!pollInit()) return;

    m_mainPageLoader->loadMainPageDataSync();
    m_recentPages          = m_mainPageLoader->recentPages();
    m_groupedTreeJson      = m_mainPageLoader->groupedTreeJson();
    m_recentJournalLines   = m_mainPageLoader->recentJournalLines();
    m_journalBlocks        = m_mainPageLoader->journalBlocks();
    emit data_refreshed();

    m_mainPageLoader->rebuildTreeInBackground(this);
}

bool NotesBridge::poll_main_page_data()
{
    if (!conn_) {
        if (pollInit())
            load_main_page_data();
        return false;
    }

    auto result = m_mainPageLoader->poll_main_page_data();
    if (!result.hasResult)
        return false;

    m_recentPages     = result.recentPages;
    m_groupedTreeJson = result.groupedTreeJson;
    emit data_refreshed();
    return true;
}

bool NotesBridge::poll_results()
{
    auto result = m_pageStore->poll_results();
    if (!result.hasResult)
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
/*  Export (delegated to ExportHelper)                                 */
/* ================================================================== */

QString NotesBridge::export_html(QString page_name)
{
    ensureInit();
    return m_exportHelper->export_html(page_name, m_isJournalPage);
}

QString NotesBridge::export_all_html()
{
    ensureInit();
    return m_exportHelper->export_all_html();
}

QString NotesBridge::open_in_browser(QString page_name)
{
    return m_exportHelper->open_in_browser(page_name, m_isJournalPage);
}

/* ================================================================== */
/*  Server (delegated to ServerManager)                                */
/* ================================================================== */

QString NotesBridge::get_server_urls_json()
{
    return m_serverManager->get_server_urls_json();
}

QString NotesBridge::start_web_server()
{
    ensureInit();
    QString url = m_serverManager->start_web_server();
    m_webServerRunning = m_serverManager->isRunning();
    m_webServerUrl     = m_serverManager->primaryUrl();
    emit web_server_status_changed();
    return url;
}

void NotesBridge::stop_web_server()
{
    m_serverManager->stop_web_server();
    m_webServerRunning = false;
    m_webServerUrl.clear();
    emit web_server_status_changed();
}

bool NotesBridge::toggle_web_server()
{
    bool running = m_serverManager->toggle_web_server();
    m_webServerRunning = m_serverManager->isRunning();
    m_webServerUrl     = m_serverManager->primaryUrl();
    emit web_server_status_changed();
    return running;
}

void NotesBridge::configure_ai(QString provider, QString url, QString model,
                               QString key, int timeout, bool auto_read,
                               bool auto_create, bool require_edit,
                               bool allow_self_signed, bool allow_fetch)
{
    m_serverManager->configure_ai(provider, url, model, key, timeout,
                                  auto_read, auto_create, require_edit,
                                  allow_self_signed, allow_fetch);
}

/* ================================================================== */
/*  TLS (delegated to ServerManager)                                   */
/* ================================================================== */

QString NotesBridge::install_tls_certificate(QString cert_pem_or_path,
                                             QString key_pem_or_path)
{
    ensureInit();
    QString err = m_serverManager->install_tls_certificate(
        cert_pem_or_path, key_pem_or_path);
    m_webServerRunning = m_serverManager->isRunning();
    m_webServerUrl     = m_serverManager->primaryUrl();
    emit web_server_status_changed();
    return err;
}

QString NotesBridge::reset_tls_certificate()
{
    ensureInit();
    QString err = m_serverManager->reset_tls_certificate();
    m_webServerRunning = m_serverManager->isRunning();
    m_webServerUrl     = m_serverManager->primaryUrl();
    emit web_server_status_changed();
    return err;
}

bool NotesBridge::is_custom_tls_certificate()
{
    return m_serverManager->is_custom_tls_certificate();
}

QString NotesBridge::get_tls_certificate_info_json()
{
    return m_serverManager->get_tls_certificate_info_json();
}

/* ================================================================== */
/*  Settings (delegated to ServerManager)                              */
/* ================================================================== */

void NotesBridge::set_drop_comments(bool drop)
{
    auto result = m_serverManager->set_drop_comments(drop);
    if (result.dropCommentsChanged) {
        m_dropComments = drop;
        emit drop_comments_changed();
        if (!m_currentPageName.isEmpty())
            m_pageStore->load_page(m_currentPageName, this);
        if (m_mainPageLoader) m_mainPageLoader->rebuildTreeInBackground(this);
    }
}

void NotesBridge::set_reject_public_networks(bool reject)
{
    if (m_serverManager->set_reject_public_networks(reject)) {
        m_rejectPublicNetworks = reject;
        emit reject_public_networks_changed();
    }
}

void NotesBridge::set_bind_address(QString addr)
{
    if (m_serverManager->set_bind_address(addr)) {
        m_bindAddress = addr;
        emit bind_address_changed();
    }
}

QString NotesBridge::get_network_interfaces_json()
{
    return m_serverManager->get_network_interfaces_json();
}

void NotesBridge::set_theme(QString colors_json)
{
    m_serverManager->set_theme(colors_json);
    m_themeColorsJson = colors_json;
}

void NotesBridge::set_session_expiry_hours(int hours)
{
    m_serverManager->set_session_expiry_hours(hours);
}

/* ================================================================== */
/*  Auth (delegated to ServerManager)                                  */
/* ================================================================== */

bool NotesBridge::check_auth_challenge()
{
    return m_serverManager->check_auth_challenge();
}

void NotesBridge::approve_auth_challenge(QString challenge_id)
{
    m_serverManager->approve_auth_challenge(challenge_id);
    m_authChallengePending = false;
    m_authChallengeId.clear();
    m_authVerificationCode.clear();
    emit auth_challenge_changed();
}

void NotesBridge::deny_auth_challenge(QString challenge_id)
{
    m_serverManager->deny_auth_challenge(challenge_id);
    m_authChallengePending = false;
    m_authChallengeId.clear();
    m_authVerificationCode.clear();
    emit auth_challenge_changed();
}

/* ================================================================== */
/*  Rendering (delegated to RenderHelper)                              */
/* ================================================================== */

QString NotesBridge::render_element_previews()
{
    return m_renderHelper->render_element_previews();
}
