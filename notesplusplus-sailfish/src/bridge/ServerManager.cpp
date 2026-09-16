/* ServerManager.cpp — Web server lifecycle, TLS, AI config, settings, auth. */

#include "ServerManager.h"

#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>

#include "../ffi/ffi_raii.h"

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
/*  Constructor                                                        */
/* ================================================================== */

ServerManager::ServerManager(const BridgeContext &ctx)
    : m_ctx(ctx)
    , m_tlsManager(ctx)
{
}

/* ================================================================== */
/*  Callbacks (delegated to SettingsManager)                           */
/* ================================================================== */

void ServerManager::set_on_page_reload_needed(std::function<void()> cb)
{
    m_settings.setOnPageReloadNeeded(std::move(cb));
}

void ServerManager::set_on_tree_rebuild_needed(std::function<void()> cb)
{
    m_settings.setOnTreeRebuildNeeded(std::move(cb));
}

/* ================================================================== */
/*  Server                                                             */
/* ================================================================== */

QString ServerManager::start_web_server()
{
    if (m_webServerRunning)
        return m_webServerUrl;

    const std::string notesDir = m_ctx.notesPath.toStdString();
    const std::string dbP      = dbPathFor(m_ctx.dataDir).toStdString();
    const std::string backupDir =
        (m_ctx.dataDir + QStringLiteral("/backups")).toStdString();
    const std::string certDir =
        (m_ctx.dataDir + QStringLiteral("/tls")).toStdString();

    /* Build config JSON for the FFI server start */
    QJsonObject config;
    config[QStringLiteral("bind_address")]          = m_settings.bindAddress();
    config[QStringLiteral("reject_public_networks")] = m_settings.rejectPublicNetworks();
    config[QStringLiteral("enable_tls")]             = true;
    config[QStringLiteral("tls_cert_path")] =
        QString::fromStdString(certDir) + QStringLiteral("/server.crt");
    config[QStringLiteral("tls_key_path")] =
        QString::fromStdString(certDir) + QStringLiteral("/server.key");

    QJsonObject llm;
    llm[QStringLiteral("provider")]        = m_aiConfig.provider;
    llm[QStringLiteral("endpoint_url")]    = m_aiConfig.endpointUrl;
    llm[QStringLiteral("model")]           = m_aiConfig.model;
    llm[QStringLiteral("api_key")]         = m_aiConfig.apiKey;
    llm[QStringLiteral("timeout_secs")]    = m_aiConfig.timeoutSecs;
    llm[QStringLiteral("allow_self_signed")] = m_aiConfig.allowSelfSigned;
    config[QStringLiteral("llm")]          = llm;

    QJsonObject perms;
    perms[QStringLiteral("auto_allow_read")]      = m_aiConfig.autoAllowRead;
    perms[QStringLiteral("auto_allow_create")]    = m_aiConfig.autoAllowCreate;
    perms[QStringLiteral("require_confirm_edit")] = m_aiConfig.requireConfirmEdit;
    perms[QStringLiteral("allow_fetch_url")]      = m_aiConfig.allowFetchUrl;
    config[QStringLiteral("permissions")]         = perms;

    QJsonObject auth;
    auth[QStringLiteral("session_expiry_secs")]   = m_settings.sessionExpirySecs();
    config[QStringLiteral("auth")]                = auth;

    const std::string configStr = QString::fromUtf8(
        QJsonDocument(config).toJson(QJsonDocument::Compact)).toStdString();

    HttpServerHandle *handle = notes_core_server_start(
        notesDir.c_str(), dbP.c_str(), backupDir.c_str(),
        notes_core_const_default_server_port(), configStr.c_str());

    if (!handle) {
        m_ctx.reportError(QStringLiteral("Failed to start web server"));
        return QString();
    }

    m_server.reset(handle);

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
    return primaryUrl;
}

void ServerManager::stop_web_server()
{
    if (m_server)
        m_server.reset();
    m_webServerRunning = false;
    m_webServerUrl.clear();
}

bool ServerManager::toggle_web_server()
{
    if (m_webServerRunning) {
        stop_web_server();
        return false;
    }
    return !start_web_server().isEmpty();
}

QString ServerManager::get_server_urls_json()
{
    if (!m_server) return QStringLiteral("[]");
    char *json = notes_core_server_urls_json(m_server.get());
    return ffiStringToQString(json);
}

void ServerManager::configure_ai(const QString &provider, const QString &url,
                                  const QString &model, const QString &key,
                                  int timeout, bool auto_read, bool auto_create,
                                  bool require_edit, bool allow_self_signed,
                                  bool allow_fetch)
{
    m_aiConfig.configure(provider, url, model, key,
                         timeout, auto_read, auto_create,
                         require_edit, allow_self_signed, allow_fetch);
}

/* ================================================================== */
/*  TLS (delegated to TlsManager)                                      */
/* ================================================================== */

QString ServerManager::install_tls_certificate(const QString &cert_pem_or_path,
                                                const QString &key_pem_or_path)
{
    QString error = m_tlsManager.install(cert_pem_or_path, key_pem_or_path);
    if (!error.isEmpty())
        return error;

    /* Restart server if running to pick up new cert */
    if (m_webServerRunning) {
        stop_web_server();
        start_web_server();
    }
    return QString();
}

QString ServerManager::reset_tls_certificate()
{
    QString error = m_tlsManager.reset();
    if (!error.isEmpty())
        return error;

    if (m_webServerRunning) {
        stop_web_server();
        start_web_server();
    }
    return QString();
}

bool ServerManager::is_custom_tls_certificate()
{
    return m_tlsManager.isCustom();
}

QString ServerManager::get_tls_certificate_info_json()
{
    return m_tlsManager.getInfoJson();
}

/* ================================================================== */
/*  Settings (delegated to SettingsManager)                            */
/* ================================================================== */

SettingsChangedResult ServerManager::set_drop_comments(bool drop)
{
    return m_settings.set_drop_comments(drop);
}

void ServerManager::set_reject_public_networks(bool reject)
{
    m_settings.set_reject_public_networks(reject);
}

void ServerManager::set_bind_address(const QString &addr)
{
    m_settings.set_bind_address(addr);
}

QString ServerManager::get_network_interfaces_json()
{
    return m_settings.get_network_interfaces_json();
}

void ServerManager::set_theme(const QString &colors_json)
{
    m_settings.set_theme(colors_json);
}

void ServerManager::set_session_expiry_hours(int hours)
{
    m_settings.set_session_expiry_hours(hours);
}

/* ================================================================== */
/*  Auth                                                               */
/* ================================================================== */

bool ServerManager::check_auth_challenge()
{
    return m_authManager.check_auth_challenge();
}

void ServerManager::approve_auth_challenge(const QString &challenge_id)
{
    m_authManager.approve_auth_challenge(challenge_id);
}

void ServerManager::deny_auth_challenge(const QString &challenge_id)
{
    m_authManager.deny_auth_challenge(challenge_id);
}
