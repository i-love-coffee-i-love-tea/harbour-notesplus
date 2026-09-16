/* SettingsManager.h — Application settings (drop-comments, bind, theme, etc.).
 *
 * Pure domain class (not a QObject). Extracted from ServerManager.
 * Owns settings state and fires callbacks when cross-domain effects
 * are needed (page reload, tree rebuild).
 */

#ifndef SETTINGSMANAGER_H
#define SETTINGSMANAGER_H

#include <QString>
#include <functional>

struct SettingsChangedResult {
    bool dropCommentsChanged = false;
};

class SettingsManager
{
public:
    SettingsManager();

    /* ---- Mutators ---- */
    SettingsChangedResult set_drop_comments(bool drop);
    void    set_reject_public_networks(bool reject);
    void    set_bind_address(const QString &addr);
    QString get_network_interfaces_json() const;
    void    set_theme(const QString &colors_json);
    void    set_session_expiry_hours(int hours);

    /* ---- Accessors (for ServerManager when building config) ---- */
    bool    dropComments()        const { return m_dropComments; }
    bool    rejectPublicNetworks() const { return m_rejectPublicNetworks; }
    QString bindAddress()         const { return m_bindAddress; }
    int     sessionExpirySecs()   const { return m_sessionExpirySecs; }

    /* ---- Callbacks for cross-domain effects ---- */
    void setOnPageReloadNeeded(std::function<void()> cb);
    void setOnTreeRebuildNeeded(std::function<void()> cb);

private:
    bool    m_dropComments         = true;
    bool    m_rejectPublicNetworks = true;
    QString m_bindAddress;
    int     m_sessionExpirySecs    = 86400;

    std::function<void()> m_onPageReloadNeeded;
    std::function<void()> m_onTreeRebuildNeeded;
};

#endif /* SETTINGSMANAGER_H */
