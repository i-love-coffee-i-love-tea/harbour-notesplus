/* AiConfig.h — AI / LLM configuration struct (header-only).
 *
 * Holds all AI-related settings that ServerManager previously owned
 * directly.  Extracted so the config can be tested and reused
 * independently of the server lifecycle.
 */

#ifndef AICONFIG_H
#define AICONFIG_H

#include <QString>

struct AiConfig {
    QString provider;
    QString endpointUrl;
    QString model;
    QString apiKey;
    int     timeoutSecs       = 90;
    bool    autoAllowRead     = false;
    bool    autoAllowCreate   = false;
    bool    requireConfirmEdit = true;
    bool    allowSelfSigned   = false;
    bool    allowFetchUrl     = false;

    void configure(const QString &prov, const QString &url,
                   const QString &mdl, const QString &key,
                   int timeout, bool auto_read, bool auto_create,
                   bool require_edit, bool allow_self_signed,
                   bool allow_fetch)
    {
        provider         = prov;
        endpointUrl      = url.trimmed();
        model            = mdl.trimmed();
        apiKey           = key.trimmed();
        timeoutSecs      = (timeout > 0) ? timeout : 90;
        autoAllowRead    = auto_read;
        autoAllowCreate  = auto_create;
        requireConfirmEdit = require_edit;
        allowSelfSigned  = allow_self_signed;
        allowFetchUrl    = allow_fetch;
    }
};

#endif /* AICONFIG_H */
