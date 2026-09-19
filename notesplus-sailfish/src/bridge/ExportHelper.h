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
    QString export_pdf(const QString &page_name, bool isJournalPage = false);
    QString export_all_pdf();
    QString get_pdf_export_path(const QString &page_name, bool isJournalPage = false);
    bool pdf_export_exists(const QString &page_name, bool isJournalPage = false);
    bool any_pdf_export_exists();
    bool render_page_to_pdf(const QString &page_path, const QString &output_path);
    QString open_in_browser(const QString &page_name, bool isJournalPage);

    /* Static helper and FFI trampoline for web server PDF exports */
    static bool render_page_to_pdf_static(const QString &notesPath, const QString &page_path, const QString &output_path);
    static int pdfExportCallback(const char *note_rel_path, const char *out_pdf_path);

private:
    const BridgeContext &m_ctx;
    std::function<bool()>               m_isServerRunning;
    std::function<QString()>            m_getServerUrl;
    std::function<QString(const QString &)> m_resolvePagePath;
};

#endif /* EXPORTHELPER_H */
