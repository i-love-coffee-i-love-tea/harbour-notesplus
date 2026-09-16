/* ServerManager.h — Web server lifecycle, TLS, AI config, settings, auth.
 *
 * Pure domain class (not a QObject). Manages the embedded web server,
 * AI/LLM configuration, and authentication challenges via FFI.
 *
 * Settings are delegated to SettingsManager; TLS to TlsManager.
 * Ownership of HttpServerPtr moves from NotesBridge to ServerManager
 * when start_web_server succeeds.
 */

#ifndef SERVERMANAGER_H
#define SERVERMANAGER_H

#include <QString>
#include <functional>

#include "AiConfig.h"
#include "AuthManager.h"
#include "BridgeContext.h"
#include "SettingsManager.h"
#include "TlsManager.h"
#include "../ffi/ffi_raii.h"

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

    /* ---- TLS (delegates to TlsManager; restarts server on success) ---- */
    QString install_tls_certificate(const QString &cert_pem_or_path,
                                    const QString &key_pem_or_path);
    QString reset_tls_certificate();
    bool    is_custom_tls_certificate();
    QString get_tls_certificate_info_json();

    /* ---- Settings (delegates to SettingsManager) ---- */
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

    /* ---- Accessors (proxy to sub-managers) ---- */
    bool    dropComments()         const { return m_settings.dropComments(); }
    bool    rejectPublicNetworks() const { return m_settings.rejectPublicNetworks(); }
    QString bindAddress()          const { return m_settings.bindAddress(); }
    bool    isRunning()            const { return m_webServerRunning; }
    QString primaryUrl()           const { return m_webServerUrl; }
    bool    authChallengePending() const { return m_authManager.isPending(); }
    QString authChallengeId()      const { return m_authManager.challengeId(); }
    QString authVerificationCode() const { return m_authManager.verificationCode(); }

    /* ---- Callbacks (delegates to SettingsManager) ---- */
    void set_on_page_reload_needed(std::function<void()> cb);
    void set_on_tree_rebuild_needed(std::function<void()> cb);

private:
    const BridgeContext &m_ctx;

    /* ---- Sub-managers ---- */
    SettingsManager m_settings;
    TlsManager      m_tlsManager;

    /* ---- FFI handle ---- */
    HttpServerPtr m_server;

    /* ---- Server state ---- */
    bool    m_webServerRunning = false;
    QString m_webServerUrl;

    /* ---- AI / LLM config (decomposed) ---- */
    AiConfig m_aiConfig;

    /* ---- Auth challenge (decomposed) ---- */
    AuthManager m_authManager;
};

#endif /* SERVERMANAGER_H */
