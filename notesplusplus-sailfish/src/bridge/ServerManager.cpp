/* ServerManager.cpp — Web server lifecycle, TLS, AI config, settings, auth. */

#include "ServerManager.h"

#include <QDir>
#include <QFile>
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
    , m_bindAddress(QStringLiteral("0.0.0.0"))
{
}

/* ================================================================== */
/*  Callbacks                                                          */
/* ================================================================== */

void ServerManager::set_on_page_reload_needed(std::function<void()> cb)
{
    m_onPageReloadNeeded = std::move(cb);
}

void ServerManager::set_on_tree_rebuild_needed(std::function<void()> cb)
{
    m_onTreeRebuildNeeded = std::move(cb);
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
    m_llmProvider        = provider;
    m_llmEndpointUrl     = url.trimmed();
    m_llmModel           = model.trimmed();
    m_llmApiKey          = key.trimmed();
    m_llmTimeoutSecs     = (timeout > 0) ? timeout : 90;
    m_autoAllowRead      = auto_read;
    m_autoAllowCreate    = auto_create;
    m_requireConfirmEdit = require_edit;
    m_allowSelfSigned    = allow_self_signed;
    m_allowFetchUrl      = allow_fetch;
}

/* ================================================================== */
/*  TLS                                                                */
/* ================================================================== */

QString ServerManager::install_tls_certificate(const QString &cert_pem_or_path,
                                                const QString &key_pem_or_path)
{
    const QString certPath = m_ctx.dataDir + QStringLiteral("/tls/server.crt");
    const QString keyPath  = m_ctx.dataDir + QStringLiteral("/tls/server.key");

    /* Ensure TLS directory exists */
    QDir().mkpath(m_ctx.dataDir + QStringLiteral("/tls"));

    char *err = notes_core_server_tls_install(
        qstrToFFI(certPath), qstrToFFI(keyPath),
        qstrToFFI(cert_pem_or_path), qstrToFFI(key_pem_or_path));

    QString error = ffiStringToQString(err);
    if (!error.isEmpty()) {
        m_ctx.reportError(QStringLiteral("Failed to install SSL certificate: ")
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

QString ServerManager::reset_tls_certificate()
{
    const QString certPath = m_ctx.dataDir + QStringLiteral("/tls/server.crt");
    const QString keyPath  = m_ctx.dataDir + QStringLiteral("/tls/server.key");

    char *err = notes_core_server_tls_reset(
        qstrToFFI(certPath), qstrToFFI(keyPath));

    QString error = ffiStringToQString(err);
    if (!error.isEmpty()) {
        m_ctx.reportError(QStringLiteral("Failed to reset SSL certificate: ")
                          + error);
        return error;
    }

    if (m_webServerRunning) {
        stop_web_server();
        start_web_server();
    }
    return QString();
}

bool ServerManager::is_custom_tls_certificate()
{
    const QString certPath = m_ctx.dataDir + QStringLiteral("/tls/server.crt");
    return notes_core_server_tls_is_custom(qstrToFFI(certPath)) != 0;
}

QString ServerManager::get_tls_certificate_info_json()
{
    const QString certPath = m_ctx.dataDir + QStringLiteral("/tls/server.crt");
    const QString keyPath  = m_ctx.dataDir + QStringLiteral("/tls/server.key");
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

SettingsChangedResult ServerManager::set_drop_comments(bool drop)
{
    SettingsChangedResult result;
    if (m_dropComments != drop) {
        m_dropComments = drop;
        result.dropCommentsChanged = true;
        if (m_onPageReloadNeeded)
            m_onPageReloadNeeded();
        if (m_onTreeRebuildNeeded)
            m_onTreeRebuildNeeded();
    }
    return result;
}

void ServerManager::set_reject_public_networks(bool reject)
{
    m_rejectPublicNetworks = reject;
}

void ServerManager::set_bind_address(const QString &addr)
{
    m_bindAddress = addr;
}

QString ServerManager::get_network_interfaces_json()
{
    char *json = notes_core_get_network_interfaces_json();
    return ffiStringToQString(json);
}

void ServerManager::set_theme(const QString &colors_json)
{
    Q_UNUSED(colors_json);
    /* Value stored by the Facade (for the BridgeContext callback).
     * ServerManager only needs to know the theme is refreshed. */
}

void ServerManager::set_session_expiry_hours(int hours)
{
    m_sessionExpirySecs = (hours > 0 ? hours : 1) * 3600;
}

/* ================================================================== */
/*  Auth                                                               */
/* ================================================================== */

bool ServerManager::check_auth_challenge()
{
    return m_authChallengePending;
}

void ServerManager::approve_auth_challenge(const QString &challenge_id)
{
    Q_UNUSED(challenge_id);
    m_authChallengePending = false;
    m_authChallengeId.clear();
    m_authVerificationCode.clear();
}

void ServerManager::deny_auth_challenge(const QString &challenge_id)
{
    Q_UNUSED(challenge_id);
    m_authChallengePending = false;
    m_authChallengeId.clear();
    m_authVerificationCode.clear();
}
