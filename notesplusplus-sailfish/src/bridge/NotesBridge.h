/* NotesBridge.h — C++ QObject bridge for Notes++ core functionality.
 *
 * This is the C++ port of the Rust NotesBridge (mod.rs + pages.rs +
 * server_bridge.rs + journal_bridge.rs + search_bridge.rs).
 * QML-facing property names, signal names, and method names are
 * IDENTICAL to the Rust version.
 */

#ifndef NOTESBRIDGE_H
#define NOTESBRIDGE_H

#include <QObject>
#include <QString>
#include <QVariantList>
#include <QJsonObject>
#include <QJsonDocument>
#include <QJsonArray>
#include <QNetworkConfigurationManager>
#include <QtConcurrent/QtConcurrent>
#include <mutex>
#include <condition_variable>
#include <atomic>

#include "BlockListModel.h"
#include "BridgeContext.h"
#include "RenderHelper.h"
#include "ExportHelper.h"
#include "PageStore.h"
#include "BlockEditor.h"
#include "JournalStore.h"
#include "GroupManager.h"
#include "MainPageLoader.h"
#include "SearchManager.h"
#include "ServerManager.h"
#include "../ffi/ffi_raii.h"

#include <memory>

class NotesBridge : public QObject
{
    Q_OBJECT

    /* ---- Properties (28) — exact names match Rust bridge ---- */
    Q_PROPERTY(QString     current_page_name         READ currentPageName            NOTIFY page_changed)
    Q_PROPERTY(QString     current_page_group_path   READ currentPageGroupPath       NOTIFY current_page_group_path_changed)
    Q_PROPERTY(QString     current_page_full_path    READ currentPageFullPath        NOTIFY current_page_full_path_changed)
    Q_PROPERTY(QString     current_page_file_path    READ currentPageFilePath        NOTIFY page_changed)
    Q_PROPERTY(QVariantList current_blocks           READ currentBlocks              NOTIFY page_changed)
    Q_PROPERTY(BlockListModel* block_model           READ blockModel                 CONSTANT)
    Q_PROPERTY(bool        is_journal_page           READ isJournalPage              NOTIFY page_changed)
    Q_PROPERTY(int         blocks_version            READ blocksVersion              NOTIFY page_changed)
    Q_PROPERTY(QString     notes_dir                 MEMBER m_notesDir               NOTIFY page_changed)
    Q_PROPERTY(QString     search_query              READ searchQuery                NOTIFY search_results_changed)
    Q_PROPERTY(QVariantList search_results           READ searchResults              NOTIFY search_results_changed)
    Q_PROPERTY(bool        search_loading            READ searchLoading              NOTIFY loading_changed)
    Q_PROPERTY(QVariantList recent_pages             READ recentPages                NOTIFY data_refreshed)
    Q_PROPERTY(QString     grouped_tree_json         READ groupedTreeJson            NOTIFY data_refreshed)
    Q_PROPERTY(int         group_display_depth       READ groupDisplayDepth          NOTIFY group_depth_changed)
    Q_PROPERTY(QVariantList recent_journal_lines     READ recentJournalLines         NOTIFY data_refreshed)
    Q_PROPERTY(QVariantList journal_blocks           READ journalBlocks              NOTIFY data_refreshed)
    Q_PROPERTY(bool        is_loading                READ isLoading                  NOTIFY loading_changed)
    Q_PROPERTY(bool        drop_comments             READ dropComments               NOTIFY drop_comments_changed)
    Q_PROPERTY(bool        reject_public_networks    READ rejectPublicNetworks       NOTIFY reject_public_networks_changed)
    Q_PROPERTY(QString     bind_address              READ bindAddress                NOTIFY bind_address_changed)
    Q_PROPERTY(bool        web_server_running        READ webServerRunning           NOTIFY web_server_status_changed)
    Q_PROPERTY(QString     web_server_url            READ webServerUrl               NOTIFY web_server_status_changed)
    Q_PROPERTY(QString     error_message             MEMBER m_errorMessage           NOTIFY error_occurred)
    Q_PROPERTY(bool        initialized               MEMBER m_initialized            NOTIFY initialized_changed)
    Q_PROPERTY(bool        auth_challenge_pending    READ authChallengePending       NOTIFY auth_challenge_changed)
    Q_PROPERTY(QString     auth_challenge_id         READ authChallengeId            NOTIFY auth_challenge_changed)
    Q_PROPERTY(QString     auth_verification_code    READ authVerificationCode       NOTIFY auth_challenge_changed)

public:
    explicit NotesBridge(QObject *parent = nullptr);
    ~NotesBridge() override;

    BlockListModel* blockModel() const { return m_blockListModel; }

    /* ---- Delegating READ accessors ---- */
    QString      currentPageName()      const { return m_pageStore->currentPageName(); }
    QString      currentPageGroupPath() const { return m_pageStore->currentPageGroupPath(); }
    QString      currentPageFullPath()  const { return m_pageStore->currentPageFullPath(); }
    QString      currentPageFilePath()  const { return m_pageStore->currentPageFilePath(); }
    QVariantList currentBlocks()        const { return m_pageStore->currentBlocks(); }
    bool         isJournalPage()        const { return m_pageStore->isJournalPage(); }
    int          blocksVersion()        const { return m_pageStore->blocksVersion(); }
    bool         isLoading()            const { return m_pageStore->isLoading(); }
    QString      searchQuery()          const { return m_searchManager->searchQuery(); }
    QVariantList searchResults()        const { return m_searchManager->searchResults(); }
    bool         searchLoading()        const { return m_searchManager->searchLoading(); }
    QVariantList recentPages()          const { return m_mainPageLoader->recentPages(); }
    QString      groupedTreeJson()      const { return m_mainPageLoader->groupedTreeJson(); }
    QVariantList recentJournalLines()   const { return m_mainPageLoader->recentJournalLines(); }
    QVariantList journalBlocks()        const { return m_mainPageLoader->journalBlocks(); }
    int          groupDisplayDepth()    const { return m_groupManager->group_display_depth(); }
    bool         dropComments()         const { return m_serverManager->dropComments(); }
    bool         rejectPublicNetworks() const { return m_serverManager->rejectPublicNetworks(); }
    QString      bindAddress()          const { return m_serverManager->bindAddress(); }
    bool         webServerRunning()     const { return m_serverManager->isRunning(); }
    QString      webServerUrl()         const { return m_serverManager->primaryUrl(); }
    bool         authChallengePending() const { return m_serverManager->authChallengePending(); }
    QString      authChallengeId()      const { return m_serverManager->authChallengeId(); }
    QString      authVerificationCode() const { return m_serverManager->authVerificationCode(); }

    /* ---- Q_INVOKABLE methods (54) — exact names match Rust bridge ---- */

    // Page operations
    Q_INVOKABLE void    load_page(QString name);
    Q_INVOKABLE void    save_block(int index, QString raw_text);
    Q_INVOKABLE void    save_block_range(int start_index, int count, QString raw_text);
    Q_INVOKABLE void    append_to_current_page(QString text, bool is_task);
    Q_INVOKABLE void    save_journal_block(int index, QString raw_text);
    Q_INVOKABLE void    toggle_journal_checkbox(int block_index, QString item_path);
    Q_INVOKABLE void    append_to_journal(QString text, bool is_task);
    Q_INVOKABLE QString get_page_source(QString name);
    Q_INVOKABLE void    save_page_source(QString name, QString content);
    Q_INVOKABLE void    create_page(QString name, QString color = QString());
    Q_INVOKABLE bool    set_page_color(QString name, QString color);
    Q_INVOKABLE void    delete_page(QString name);
    Q_INVOKABLE bool    rename_page(QString old_path, QString new_title);
    Q_INVOKABLE void    navigate_to_page(QString name);
    Q_INVOKABLE void    insert_link_at_cursor(int block_idx, int cursor_pos, QString target);
    Q_INVOKABLE void    toggle_checkbox(int block_index, QString item_path);

    // Group operations
    Q_INVOKABLE bool    create_group(QString parent_path, QString name);
    Q_INVOKABLE bool    rename_group(QString old_path, QString new_name);
    Q_INVOKABLE bool    delete_group(QString path, bool recursive);
    Q_INVOKABLE bool    move_page_to_group(QString page_full_path, QString target_group);
    Q_INVOKABLE void    set_group_display_depth(int depth);
    Q_INVOKABLE bool    toggle_group_collapsed(QString group_path);
    Q_INVOKABLE bool    set_group_note_sort(QString group_path, QString note_sort);
    Q_INVOKABLE QString get_group_note_sort(QString group_path);
    Q_INVOKABLE QString get_groups_json();

    // Index
    Q_INVOKABLE QString rebuild_index();

    // Search
    Q_INVOKABLE void    do_search(QString query);
    Q_INVOKABLE void    search(QString query);
    Q_INVOKABLE bool    poll_search();
    Q_INVOKABLE bool    poll_search_previews();
    Q_INVOKABLE QString get_linkable_pages_json(QString query);

    // Main page data
    Q_INVOKABLE void    load_main_page_data();
    Q_INVOKABLE bool    poll_main_page_data();
    Q_INVOKABLE bool    poll_results();

    // Export
    Q_INVOKABLE QString export_html(QString page_name);
    Q_INVOKABLE QString export_all_html();
    Q_INVOKABLE QString open_in_browser(QString page_name);

    // Server
    Q_INVOKABLE QString get_server_urls_json();
    Q_INVOKABLE QString start_web_server();
    Q_INVOKABLE void    stop_web_server();
    Q_INVOKABLE bool    toggle_web_server();
    Q_INVOKABLE void    configure_ai(QString provider, QString url, QString model,
                                     QString key, int timeout, bool auto_read,
                                     bool auto_create, bool require_edit,
                                     bool allow_self_signed, bool allow_fetch);

    // TLS
    Q_INVOKABLE QString install_tls_certificate(QString cert_pem_or_path, QString key_pem_or_path);
    Q_INVOKABLE QString reset_tls_certificate();
    Q_INVOKABLE bool    is_custom_tls_certificate();
    Q_INVOKABLE QString get_tls_certificate_info_json();

    // Settings
    Q_INVOKABLE void    set_drop_comments(bool drop);
    Q_INVOKABLE void    set_reject_public_networks(bool reject);
    Q_INVOKABLE void    set_bind_address(QString addr);
    Q_INVOKABLE QString get_network_interfaces_json();
    Q_INVOKABLE void    set_theme(QString colors_json);
    Q_INVOKABLE void    set_session_expiry_hours(int hours);

    // Auth
    Q_INVOKABLE bool    check_auth_challenge();
    Q_INVOKABLE void    approve_auth_challenge(QString challenge_id);
    Q_INVOKABLE void    deny_auth_challenge(QString challenge_id);

    // Lifecycle
    Q_INVOKABLE void    poll_init_and_load();

    // Rendering
    Q_INVOKABLE QString render_element_previews();

signals:
    /* ---- Signals (14) — exact names match Rust bridge ---- */
    void page_changed();
    void current_page_group_path_changed();
    void current_page_full_path_changed();
    void search_results_changed();
    void data_refreshed();
    void group_depth_changed();
    void loading_changed();
    void drop_comments_changed();
    void reject_public_networks_changed();
    void bind_address_changed();
    void web_server_status_changed();
    void error_occurred(QString message);
    void initialized_changed();
    void auth_challenge_changed();

private:
    /* ---- Internal helpers ---- */
    void    ensureInit();
    bool    pollInit();
    bool    ensureInitBlocking();
    void    reportError(const QString &msg);
    QString resolvePagePath(const QString &name);
    QString currentPageRelativePath();
    void   *rawConn() const;
    QString buildOptionsJson() const;
    void    loadMainPageDataSync();
    void    rebuildTreeInBackground();

    /* ---- FFI handles (RAII) ---- */
    AppPathsPtr    paths_;
    DbConnPtr      conn_;

    /* ---- Filesystem paths ---- */
    QString m_notesPath;                    // absolute notes directory
    QString m_dataDir;                      // absolute data directory

    /* ---- Background init state ---- */
    std::mutex              m_initMutex;
    std::condition_variable m_initCv;
    void                   *m_initResult = nullptr;
    bool                    m_initDone   = false;
    bool                    m_initStarted = false;

    /* ---- Theme / options cache ---- */
    QString m_themeColorsJson;

    /* ---- Property storage (4 owned by NotesBridge) ---- */
    BlockListModel* m_blockListModel = nullptr;
    QString      m_notesDir;
    QString      m_errorMessage;
    bool         m_initialized         = false;

    /* ---- Shared context (must outlive domain classes) ---- */
    BridgeContext m_ctx;

    /* ---- Domain classes (delegation targets) ---- */
    std::unique_ptr<PageStore>       m_pageStore;
    std::unique_ptr<BlockEditor>     m_blockEditor;
    std::unique_ptr<JournalStore>    m_journalStore;
    std::unique_ptr<GroupManager>    m_groupManager;
    std::unique_ptr<MainPageLoader>  m_mainPageLoader;
    std::unique_ptr<SearchManager>   m_searchManager;
    std::unique_ptr<ServerManager>   m_serverManager;
    std::unique_ptr<RenderHelper>    m_renderHelper;
    std::unique_ptr<ExportHelper>    m_exportHelper;

    /* ---- Lifetime guard for detached threads ---- */
    std::shared_ptr<std::atomic<bool>> m_alive;

    /* ---- Network configuration monitor ---- */
    QNetworkConfigurationManager *m_netConfigManager = nullptr;

    Q_DISABLE_COPY(NotesBridge)
};

#endif /* NOTESBRIDGE_H */
