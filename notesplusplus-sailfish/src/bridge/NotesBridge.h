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
#include <QtConcurrent/QtConcurrent>
#include <mutex>
#include <condition_variable>
#include <optional>
#include <thread>
#include <atomic>

#include "BlockListModel.h"
#include "../ffi/ffi_raii.h"

class NotesBridge : public QObject
{
    Q_OBJECT

    /* ---- Properties (28) — exact names match Rust bridge ---- */
    Q_PROPERTY(QString     current_page_name         MEMBER m_currentPageName         NOTIFY page_changed)
    Q_PROPERTY(QString     current_page_group_path   MEMBER m_currentPageGroupPath    NOTIFY current_page_group_path_changed)
    Q_PROPERTY(QString     current_page_full_path    MEMBER m_currentPageFullPath     NOTIFY current_page_full_path_changed)
    Q_PROPERTY(QString     current_page_file_path    MEMBER m_currentPageFilePath     NOTIFY page_changed)
    Q_PROPERTY(QVariantList current_blocks           MEMBER m_currentBlocks           NOTIFY page_changed)
    Q_PROPERTY(BlockListModel* block_model           READ   blockModel                CONSTANT)
    Q_PROPERTY(bool        is_journal_page           MEMBER m_isJournalPage           NOTIFY page_changed)
    Q_PROPERTY(int         blocks_version            MEMBER m_blocksVersion           NOTIFY page_changed)
    Q_PROPERTY(QString     notes_dir                 MEMBER m_notesDir                NOTIFY page_changed)
    Q_PROPERTY(QString     search_query              MEMBER m_searchQuery             NOTIFY search_results_changed)
    Q_PROPERTY(QVariantList search_results           MEMBER m_searchResults           NOTIFY search_results_changed)
    Q_PROPERTY(bool        search_loading            MEMBER m_searchLoading           NOTIFY loading_changed)
    Q_PROPERTY(QVariantList recent_pages             MEMBER m_recentPages             NOTIFY data_refreshed)
    Q_PROPERTY(QString     grouped_tree_json         MEMBER m_groupedTreeJson         NOTIFY data_refreshed)
    Q_PROPERTY(int         group_display_depth       MEMBER m_groupDisplayDepth       NOTIFY group_depth_changed)
    Q_PROPERTY(QVariantList recent_journal_lines     MEMBER m_recentJournalLines      NOTIFY data_refreshed)
    Q_PROPERTY(QVariantList journal_blocks           MEMBER m_journalBlocks           NOTIFY data_refreshed)
    Q_PROPERTY(bool        is_loading                MEMBER m_isLoading               NOTIFY loading_changed)
    Q_PROPERTY(bool        drop_comments             MEMBER m_dropComments            NOTIFY drop_comments_changed)
    Q_PROPERTY(bool        reject_public_networks    MEMBER m_rejectPublicNetworks    NOTIFY reject_public_networks_changed)
    Q_PROPERTY(QString     bind_address              MEMBER m_bindAddress             NOTIFY bind_address_changed)
    Q_PROPERTY(bool        web_server_running        MEMBER m_webServerRunning        NOTIFY web_server_status_changed)
    Q_PROPERTY(QString     web_server_url            MEMBER m_webServerUrl            NOTIFY web_server_status_changed)
    Q_PROPERTY(QString     error_message             MEMBER m_errorMessage            NOTIFY error_occurred)
    Q_PROPERTY(bool        initialized               MEMBER m_initialized             NOTIFY initialized_changed)
    Q_PROPERTY(bool        auth_challenge_pending    MEMBER m_authChallengePending    NOTIFY auth_challenge_changed)
    Q_PROPERTY(QString     auth_challenge_id         MEMBER m_authChallengeId         NOTIFY auth_challenge_changed)
    Q_PROPERTY(QString     auth_verification_code    MEMBER m_authVerificationCode    NOTIFY auth_challenge_changed)

public:
    explicit NotesBridge(QObject *parent = nullptr);
    ~NotesBridge() override;

    BlockListModel* blockModel() const { return m_blockListModel; }

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
    Q_INVOKABLE void    create_page(QString name);
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
    HttpServerPtr  server_;

    /* ---- Filesystem paths ---- */
    QString m_notesPath;                    // absolute notes directory
    QString m_dataDir;                      // absolute data directory

    /* ---- Background init state ---- */
    std::mutex              m_initMutex;
    std::condition_variable m_initCv;
    void                   *m_initResult = nullptr;
    bool                    m_initDone   = false;
    bool                    m_initStarted = false;

    /* ---- Pending page-load result (background -> poll_results) ---- */
    struct PendingPageResult {
        QVariantList blocks;
        QString      error;
    };
    std::mutex                        m_pendingMutex;
    std::optional<PendingPageResult>  m_pendingPage;

    /* ---- Pending main-page data (background -> poll_main_page_data) ---- */
    struct MainPageData {
        QStringList recentPageJsons;
        QString     groupedTreeJson;
    };
    std::mutex                    m_pendingMainMutex;
    std::optional<MainPageData>   m_pendingMainPage;

    /* ---- Pending search result (background -> poll_search) ---- */
    struct SearchHit {
        QString title, filename, groupPath, fullPath, snippet;
        QString createdAt, updatedAt;
        int     blockCount = 0;
        QString previewJson;
    };
    std::mutex                         m_pendingSearchMutex;
    std::optional<QList<SearchHit>>    m_pendingSearchHits;
    QString                            m_pendingSearchError;
    std::atomic<int>                   m_searchGeneration{0};

    /* ---- Pending search previews (background -> poll_search_previews) ---- */
    std::mutex                               m_pendingPreviewMutex;
    std::optional<QMap<QString, QString>>    m_pendingPreviews;

    /* ---- Search state for preview matching ---- */
    QStringList m_currentSearchFilenames;
    QStringList m_currentSearchJsons;

    /* ---- Theme / options cache ---- */
    QString m_themeColorsJson;

    /* ---- AI / Server config ---- */
    QString m_llmProvider;
    QString m_llmEndpointUrl;
    QString m_llmModel;
    QString m_llmApiKey;
    int     m_llmTimeoutSecs     = 90;
    bool    m_autoAllowRead      = false;
    bool    m_autoAllowCreate    = false;
    bool    m_requireConfirmEdit = true;
    bool    m_allowSelfSigned    = false;
    bool    m_allowFetchUrl      = false;
    int     m_sessionExpirySecs  = 86400;

    /* ---- Property storage (28) ---- */
    QString      m_currentPageName;
    QString      m_currentPageGroupPath;
    QString      m_currentPageFullPath;
    QString      m_currentPageFilePath;
    QVariantList m_currentBlocks;
    BlockListModel* m_blockListModel = nullptr;
    bool         m_isJournalPage       = false;
    int          m_blocksVersion       = 0;
    QString      m_notesDir;
    QString      m_searchQuery;
    QVariantList m_searchResults;
    bool         m_searchLoading       = false;
    QVariantList m_recentPages;
    QString      m_groupedTreeJson;
    int          m_groupDisplayDepth   = 2;
    QVariantList m_recentJournalLines;
    QVariantList m_journalBlocks;
    bool         m_isLoading           = false;
    bool         m_dropComments        = true;
    bool         m_rejectPublicNetworks = true;
    QString      m_bindAddress;
    bool         m_webServerRunning    = false;
    QString      m_webServerUrl;
    QString      m_errorMessage;
    bool         m_initialized         = false;
    bool         m_authChallengePending = false;
    QString      m_authChallengeId;
    QString      m_authVerificationCode;

    /* ---- Lifetime guard for detached threads ---- */
    std::shared_ptr<std::atomic<bool>> m_alive;

    Q_DISABLE_COPY(NotesBridge)
};

#endif /* NOTESBRIDGE_H */
