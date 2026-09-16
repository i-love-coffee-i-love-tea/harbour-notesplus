/* ServerManager.h — Web server lifecycle, TLS, AI config, settings, auth.
 *
 * Pure domain class (not a QObject). Manages the embedded web server,
 * TLS certificates, AI/LLM configuration, application settings, and
 * authentication challenges via FFI.
 *
 * Ownership of HttpServerPtr moves from NotesBridge to ServerManager
 * when start_web_server succeeds.
 */

#ifndef SERVERMANAGER_H
#define SERVERMANAGER_H

#include <QString>
#include <functional>

#include "BridgeContext.h"
#include "../ffi/ffi_raii.h"

struct SettingsChangedResult {
    bool dropCommentsChanged = false;
};

class ServerManager
{
public:
    explicit ServerManager(const BridgeContext &ctx);

    /* ---- Server ---- */
    QString start_web_server();
    void    stop_web_server();
    bool    toggle_web_server();
    QString get_server_urls_json();
    void    configure_ai(const QString &provider, const QString &url,
                         const QString &model, const QString &key,
                         int timeout, bool auto_read, bool auto_create,
                         bool require_edit, bool allow_self_signed,
                         bool allow_fetch);

    /* ---- TLS ---- */
    QString install_tls_certificate(const QString &cert_pem_or_path,
                                    const QString &key_pem_or_path);
    QString reset_tls_certificate();
    bool    is_custom_tls_certificate();
    QString get_tls_certificate_info_json();

    /* ---- Settings ---- */
    SettingsChangedResult set_drop_comments(bool drop);
    void    set_reject_public_networks(bool reject);
    void    set_bind_address(const QString &addr);
    QString get_network_interfaces_json();
    void    set_theme(const QString &colors_json);
    void    set_session_expiry_hours(int hours);

    /* ---- Auth ---- */
    bool    check_auth_challenge();
    void    approve_auth_challenge(const QString &challenge_id);
    void    deny_auth_challenge(const QString &challenge_id);

    /* ---- Callbacks (set by Facade) ---- */
    void set_on_page_reload_needed(std::function<void()> cb);
    void set_on_tree_rebuild_needed(std::function<void()> cb);

private:
    const BridgeContext &m_ctx;

    /* ---- FFI handle ---- */
    HttpServerPtr m_server;

    /* ---- Server state ---- */
    bool    m_webServerRunning = false;
    QString m_webServerUrl;

    /* ---- AI / LLM config ---- */
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

    /* ---- Session ---- */
    int m_sessionExpirySecs = 86400;

    /* ---- Settings ---- */
    bool    m_dropComments        = true;
    bool    m_rejectPublicNetworks = true;
    QString m_bindAddress;

    /* ---- Auth challenge ---- */
    bool    m_authChallengePending = false;
    QString m_authChallengeId;
    QString m_authVerificationCode;

    /* ---- Callbacks ---- */
    std::function<void()> m_onPageReloadNeeded;
    std::function<void()> m_onTreeRebuildNeeded;
};

#endif /* SERVERMANAGER_H */
