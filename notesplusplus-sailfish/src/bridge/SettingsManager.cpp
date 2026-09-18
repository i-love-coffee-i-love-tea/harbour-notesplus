/* SettingsManager.cpp — Application settings. */

#include "SettingsManager.h"
#include "NetworkHelper.h"

#include "../ffi/ffi_raii.h"

/* ================================================================== */
/*  Constructor                                                        */
/* ================================================================== */

SettingsManager::SettingsManager()
    : m_bindAddress(QStringLiteral("0.0.0.0"))
{
}

/* ================================================================== */
/*  Callbacks                                                          */
/* ================================================================== */

void SettingsManager::setOnPageReloadNeeded(std::function<void()> cb)
{
    m_onPageReloadNeeded = std::move(cb);
}

void SettingsManager::setOnTreeRebuildNeeded(std::function<void()> cb)
{
    m_onTreeRebuildNeeded = std::move(cb);
}

/* ================================================================== */
/*  Settings                                                           */
/* ================================================================== */

SettingsChangedResult SettingsManager::set_drop_comments(bool drop)
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

void SettingsManager::set_reject_public_networks(bool reject)
{
    m_rejectPublicNetworks = reject;
}

void SettingsManager::set_bind_address(const QString &addr)
{
    m_bindAddress = addr;
}

QString SettingsManager::get_network_interfaces_json() const
{
    return NetworkHelper::getNetworkInterfacesJson();
}

void SettingsManager::set_theme(const QString &colors_json)
{
    Q_UNUSED(colors_json);
    /* Value stored by the Facade (for the BridgeContext callback).
     * SettingsManager only needs to know the theme is refreshed. */
}

void SettingsManager::set_session_expiry_hours(int hours)
{
    m_sessionExpirySecs = (hours > 0 ? hours : 1) * 3600;
}
