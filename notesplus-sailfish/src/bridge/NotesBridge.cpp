/* NotesBridge.cpp — C++ QObject bridge for Notes++ core functionality.
 *
 * Implementation of the NotesBridge class, porting the Rust NotesBridge
 * (mod.rs + pages.rs + server_bridge.rs + journal_bridge.rs + search_bridge.rs)
 * to C++ using the FFI in notesplus_core.h.
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
    , m_themeColorsJson(QStringLiteral("{\"highlightColor\":\"#0088cc\",\"primaryColor\":\"#ffffff\",\"highlightBackgroundColor\":\"rgba(0,136,204,0.25)\"}"))
    , m_blockListModel(new BlockListModel(this))
    , m_alive(std::make_shared<std::atomic<bool>>(true))
{
    m_notesPath = ffiStringToQString(
        notes_core_app_paths_notes_dir(paths_.get()));
    m_dataDir = ffiStringToQString(
        notes_core_app_paths_data_dir(paths_.get()));

    /* Default property values (mirror Rust Default::default()) */
    m_notesDir        = m_notesPath;

    /* Construct BridgeContext (must be a member so domain class refs survive). */
    m_ctx = BridgeContext{
        [this]() -> void* { return conn_.get(); },
        m_notesPath,
        m_dataDir,
        m_alive,
        [this]() -> bool { return m_serverManager->dropComments(); },
        [this]() -> QString { return m_themeColorsJson; },
        [this](const QString &msg) { reportError(msg); },
        [this]() -> QString { return buildOptionsJson(); }
    };

    /* Domain classes */
    m_pageStore = std::make_unique<PageStore>(m_ctx, m_blockListModel);
    m_pageStore->setRebuildTreeCallback([this]() {
        if (m_mainPageLoader) m_mainPageLoader->rebuildTreeInBackground(this);
    });

    m_blockEditor = std::make_unique<BlockEditor>(
        m_ctx, m_blockListModel, m_pageStore->pathResolver());

    m_journalStore = std::make_unique<JournalStore>(m_ctx);
    m_journalStore->setRebuildCallback([this]() {
        if (m_mainPageLoader) m_mainPageLoader->rebuildTreeInBackground(this);
    });
    m_journalStore->setReloadCallback([this]() {
        if (m_mainPageLoader) m_mainPageLoader->loadMainPageDataSync();
    });

    m_groupManager = std::make_unique<GroupManager>(m_ctx);
    m_groupManager->set_rebuild_tree_callback([this]() {
        if (m_mainPageLoader) m_mainPageLoader->rebuildTreeInBackground(this);
    });

    m_mainPageLoader = std::make_unique<MainPageLoader>(m_ctx);

    m_searchManager = std::make_unique<SearchManager>(m_ctx);

    m_serverManager = std::make_unique<ServerManager>(m_ctx);
    m_serverManager->set_on_page_reload_needed([this]() {
        if (!m_pageStore->currentPageName().isEmpty())
            m_pageStore->load_page(m_pageStore->currentPageName(), this);
    });
    m_serverManager->set_on_tree_rebuild_needed([this]() {
        if (m_mainPageLoader) m_mainPageLoader->rebuildTreeInBackground(this);
    });

    m_renderHelper = std::make_unique<RenderHelper>(m_ctx);

    m_exportHelper = std::make_unique<ExportHelper>(
        m_ctx,
        [this]() -> bool { return m_serverManager->isRunning(); },
        [this]() -> QString { return m_serverManager->primaryUrl(); },
        [this](const QString &name) -> QString { return resolvePagePath(name); }
    );

    /* Monitor network configuration changes to update server URL dynamically */
    m_netConfigManager = new QNetworkConfigurationManager(this);
    connect(m_netConfigManager, &QNetworkConfigurationManager::configurationChanged,
            this, [this](const QNetworkConfiguration &) {
        if (m_serverManager && m_serverManager->isRunning()) {
            emit web_server_status_changed();
        }
    });
    connect(m_netConfigManager, &QNetworkConfigurationManager::onlineStateChanged,
            this, [this](bool) {
        if (m_serverManager && m_serverManager->isRunning()) {
            emit web_server_status_changed();
        }
    });

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
            /* Copy bundled documentation and examples to user notes directory */
            const QString installedExamples = QStringLiteral("/usr/share/harbour-notesplus/examples");
            if (QDir(installedExamples).exists()) {
                notes_core_copy_examples(raw, notesDir.c_str(), installedExamples.toUtf8().constData());
            }
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
    return m_pageStore->pathResolver().resolve(name, m_pageStore->currentPageGroupPath());
}

QString NotesBridge::currentPageRelativePath()
{
    return m_pageStore->pathResolver().currentRelative(
        m_pageStore->currentPageName(), m_pageStore->currentPageGroupPath(),
        m_pageStore->currentPageFullPath(), m_pageStore->isJournalPage());
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
    emit loading_changed();
    emit current_page_group_path_changed();
    emit current_page_full_path_changed();
    emit page_changed();
}

void NotesBridge::save_block(int index, QString raw_text)
{
    save_block_range(index, 1, raw_text);
}

void NotesBridge::save_block_range(int start_index, int count, QString raw_text)
{
    if (!ensureInitBlocking()) return;
    m_blockEditor->save_block_range(start_index, count, raw_text, this,
                                    m_pageStore.get());
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
    m_journalStore->save_journal_block(index, raw_text);
    m_mainPageLoader->loadMainPageDataSync();
    m_mainPageLoader->rebuildTreeInBackground(this);
}

void NotesBridge::toggle_journal_checkbox(int block_index, QString item_path)
{
    if (!ensureInitBlocking()) return;
    m_journalStore->toggle_journal_checkbox(block_index, item_path);
    m_mainPageLoader->loadMainPageDataSync();
    m_mainPageLoader->rebuildTreeInBackground(this);
}

void NotesBridge::append_to_journal(QString text, bool is_task)
{
    ensureInit();
    m_journalStore->append_to_journal(text, is_task);
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

void NotesBridge::create_page(QString name, QString color)
{
    if (!ensureInitBlocking()) return;
    m_pageStore->create_page(name, color, this);
}

bool NotesBridge::set_page_color(QString name, QString color)
{
    if (!ensureInitBlocking()) return false;
    return m_pageStore->set_page_color(name, color);
}

void NotesBridge::delete_page(QString name)
{
    if (!ensureInitBlocking()) return;
    m_pageStore->delete_page(name, this);
    emit current_page_group_path_changed();
    emit current_page_full_path_changed();
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
    m_blockEditor->insert_link_at_cursor(block_idx, cursor_pos, target, this,
                                         m_pageStore.get());
}

void NotesBridge::toggle_checkbox(int block_index, QString item_path)
{
    if (!ensureInitBlocking()) return;
    m_blockEditor->toggle_checkbox(block_index, item_path, this,
                                   m_pageStore.get());
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
        page_full_path, target_group, m_pageStore->currentPageFullPath());
    if (!result.success) return false;

    if (result.currentPageMoved) {
        m_pageStore->updatePagePaths(result.newGroupPath,
                                     result.newFullPath,
                                     result.newFilePath);
        emit current_page_group_path_changed();
        emit current_page_full_path_changed();
        emit page_changed();
    }
    return true;
}

void NotesBridge::set_group_display_depth(int depth)
{
    if (m_groupManager->set_group_display_depth(depth)) {
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
    m_searchManager->do_search(query, this);
    emit search_results_changed();
    emit loading_changed();
}

void NotesBridge::search(QString query)
{
    m_searchManager->search(query);
    emit search_results_changed();
    emit loading_changed();
}

bool NotesBridge::poll_search()
{
    auto result = m_searchManager->poll_search();
    if (!result.hasResult) return false;

    emit loading_changed();

    if (!result.error.isEmpty()) {
        reportError(result.error);
        return true;
    }

    emit search_results_changed();
    return true;
}

bool NotesBridge::poll_search_previews()
{
    auto result = m_searchManager->poll_search_previews();
    if (!result.hasResult) return false;

    if (!result.results.isEmpty()) {
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
    emit data_refreshed();
}

void NotesBridge::load_main_page_data()
{
    ensureInit();
    if (!pollInit()) return;

    m_mainPageLoader->loadMainPageDataSync();
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

    emit data_refreshed();
    return true;
}

bool NotesBridge::poll_results()
{
    auto result = m_pageStore->poll_results();
    if (!result.hasResult)
        return false;

    emit loading_changed();

    if (!result.error.isEmpty()) {
        reportError(result.error);
        return true;
    }

    emit page_changed();
    return true;
}

/* ================================================================== */
/*  Export (delegated to ExportHelper)                                 */
/* ================================================================== */

QString NotesBridge::export_html(QString page_name)
{
    ensureInit();
    return m_exportHelper->export_html(page_name, m_pageStore->isJournalPage());
}

QString NotesBridge::export_all_html()
{
    ensureInit();
    return m_exportHelper->export_all_html();
}

QString NotesBridge::export_pdf(QString page_name)
{
    ensureInit();
    return m_exportHelper->export_pdf(page_name, m_pageStore->isJournalPage());
}

QString NotesBridge::export_all_pdf()
{
    ensureInit();
    return m_exportHelper->export_all_pdf();
}

QString NotesBridge::get_pdf_export_path(QString page_name)
{
    ensureInit();
    return m_exportHelper ? m_exportHelper->get_pdf_export_path(page_name, m_pageStore->isJournalPage()) : QString();
}

bool NotesBridge::pdf_export_exists(QString page_name)
{
    ensureInit();
    return m_exportHelper ? m_exportHelper->pdf_export_exists(page_name, m_pageStore->isJournalPage()) : false;
}

bool NotesBridge::any_pdf_export_exists()
{
    ensureInit();
    return m_exportHelper ? m_exportHelper->any_pdf_export_exists() : false;
}

QString NotesBridge::open_in_browser(QString page_name)
{
    return m_exportHelper->open_in_browser(page_name, m_pageStore->isJournalPage());
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
    emit web_server_status_changed();
    return url;
}

void NotesBridge::stop_web_server()
{
    m_serverManager->stop_web_server();
    emit web_server_status_changed();
}

bool NotesBridge::toggle_web_server()
{
    bool running = m_serverManager->toggle_web_server();
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
    emit web_server_status_changed();
    return err;
}

QString NotesBridge::reset_tls_certificate()
{
    ensureInit();
    QString err = m_serverManager->reset_tls_certificate();
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
        emit drop_comments_changed();
    }
}

void NotesBridge::set_reject_public_networks(bool reject)
{
    m_serverManager->set_reject_public_networks(reject);
    emit reject_public_networks_changed();
    emit web_server_status_changed();
}

void NotesBridge::set_bind_address(QString addr)
{
    m_serverManager->set_bind_address(addr);
    emit bind_address_changed();
    emit web_server_status_changed();
}

QString NotesBridge::get_network_interfaces_json()
{
    return m_serverManager->get_network_interfaces_json();
}

void NotesBridge::set_theme(QString colors_json)
{
    m_themeColorsJson = colors_json;
    m_serverManager->set_theme(colors_json);
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
    emit auth_challenge_changed();
}

void NotesBridge::deny_auth_challenge(QString challenge_id)
{
    m_serverManager->deny_auth_challenge(challenge_id);
    emit auth_challenge_changed();
}

/* ================================================================== */
/*  Rendering (delegated to RenderHelper)                              */
/* ================================================================== */

QString NotesBridge::render_element_previews()
{
    return m_renderHelper->render_element_previews();
}
