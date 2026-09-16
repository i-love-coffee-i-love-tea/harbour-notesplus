/* TlsManager.h — TLS certificate management.
 *
 * Pure domain class (not a QObject). Extracted from ServerManager.
 * Handles install/reset/query of TLS certificates via FFI.
 * Does NOT restart the server — that is ServerManager's responsibility.
 */

#ifndef TLSMANAGER_H
#define TLSMANAGER_H

#include <QString>

#include "BridgeContext.h"

class TlsManager
{
public:
    explicit TlsManager(const BridgeContext &ctx);

    /// Install a custom certificate. Returns error string (empty on success).
    QString install(const QString &cert_pem_or_path,
                    const QString &key_pem_or_path);

    /// Reset to the auto-generated self-signed certificate. Returns error string.
    QString reset();

    /// True if a custom (user-supplied) certificate is installed.
    bool isCustom() const;

    /// JSON object with cert metadata (is_custom, paths, exists).
    QString getInfoJson() const;

private:
    const BridgeContext &m_ctx;
};

#endif /* TLSMANAGER_H */
