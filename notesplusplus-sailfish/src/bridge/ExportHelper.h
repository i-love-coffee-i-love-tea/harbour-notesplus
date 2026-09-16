/* ExportHelper.h — HTML export and open-in-browser.
 *
 * Handles exporting AsciiDoc pages to HTML and opening them in the browser.
 * Uses callbacks for cross-domain dependencies (server status, page path resolution).
 */

#ifndef EXPORTHELPER_H
#define EXPORTHELPER_H

#include <QString>
#include <functional>
#include "BridgeContext.h"

class ExportHelper
{
public:
    ExportHelper(const BridgeContext &ctx,
                 std::function<bool()> isServerRunning,
                 std::function<QString()> getServerUrl,
                 std::function<QString(const QString&)> resolvePagePath);

    QString export_html(const QString &page_name, bool isJournalPage);
    QString export_all_html();
    QString open_in_browser(const QString &page_name, bool isJournalPage);

private:
    const BridgeContext &m_ctx;
    std::function<bool()>               m_isServerRunning;
    std::function<QString()>            m_getServerUrl;
    std::function<QString(const QString &)> m_resolvePagePath;
};

#endif /* EXPORTHELPER_H */
