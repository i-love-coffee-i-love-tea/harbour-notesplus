/* TlsManager.cpp — TLS certificate management. */

#include "TlsManager.h"

#include <QDir>
#include <QFile>
#include <QJsonDocument>
#include <QJsonObject>

#include "../ffi/ffi_raii.h"

/* ================================================================== */
/*  Constructor                                                        */
/* ================================================================== */

TlsManager::TlsManager(const BridgeContext &ctx)
    : m_ctx(ctx)
{
}

/* ================================================================== */
/*  TLS operations                                                     */
/* ================================================================== */

QString TlsManager::install(const QString &cert_pem_or_path,
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
    return QString();
}

QString TlsManager::reset()
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
    return QString();
}

bool TlsManager::isCustom() const
{
    const QString certPath = m_ctx.dataDir + QStringLiteral("/tls/server.crt");
    return notes_core_server_tls_is_custom(qstrToFFI(certPath)) != 0;
}

QString TlsManager::getInfoJson() const
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
