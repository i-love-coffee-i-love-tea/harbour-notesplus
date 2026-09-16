/* ExportHelper.cpp — HTML export and open-in-browser. */

#include "ExportHelper.h"

#include <QDir>
#include <QJsonDocument>
#include <QJsonArray>
#include <QJsonObject>
#include <QStandardPaths>
#include <cstdlib>

#include "../ffi/ffi_raii.h"

/// Extract a title from a file path or name (strip directory + .adoc suffix).
static QString extractTitle(const QString &name)
{
    QString t = name.section(QLatin1Char('/'), -1);
    if (t.endsWith(QLatin1String(".adoc")))
        t.chop(5);
    return t;
}

ExportHelper::ExportHelper(const BridgeContext &ctx,
                           std::function<bool()> isServerRunning,
                           std::function<QString()> getServerUrl,
                           std::function<QString(const QString &)> resolvePagePath)
    : m_ctx(ctx)
    , m_isServerRunning(std::move(isServerRunning))
    , m_getServerUrl(std::move(getServerUrl))
    , m_resolvePagePath(std::move(resolvePagePath))
{
}

QString ExportHelper::export_html(const QString &page_name, bool isJournalPage)
{
    QString fullPath;
    if (isJournalPage
        || page_name.compare(QLatin1String("Journal"), Qt::CaseInsensitive) == 0)
        fullPath = QStringLiteral("journal.adoc");
    else
        fullPath = m_resolvePagePath(page_name);

    const QString home = QString::fromLocal8Bit(qgetenv("HOME"));
    if (home.isEmpty()) return QString();
    const QString exportDir = home
        + QStringLiteral("/Documents/Notes++ Exports");
    QDir().mkpath(exportDir);

    const QString title = extractTitle(page_name);
    const QString outputPath = exportDir + QLatin1Char('/') + title
        + QLatin1String(".html");
    const QString absPath = m_ctx.notesPath + QLatin1Char('/') + fullPath;

    char *result = notes_core_export_html5(
        qstrToFFI(m_ctx.notesPath), qstrToFFI(fullPath),
        qstrToFFI(absPath), qstrToFFI(outputPath));

    QString path = ffiStringToQString(result);
    if (path.isEmpty())
        m_ctx.reportError(QStringLiteral("Export failed"));
    return path;
}

QString ExportHelper::export_all_html()
{
    const QString home = QString::fromLocal8Bit(qgetenv("HOME"));
    if (home.isEmpty()) return QString();
    const QString exportDir = home
        + QStringLiteral("/Documents/Notes++ Exports");
    QDir().mkpath(exportDir);

    char *result = notes_core_export_all_html5(
        qstrToFFI(m_ctx.notesPath), qstrToFFI(exportDir));
    return ffiStringToQString(result);
}

QString ExportHelper::open_in_browser(const QString &page_name,
                                      bool isJournalPage)
{
    if (m_isServerRunning()) {
        QString base = m_getServerUrl();

        QString fullPath;
        if (isJournalPage
            || page_name.compare(QLatin1String("Journal"),
                                 Qt::CaseInsensitive) == 0)
            fullPath = QStringLiteral("journal.adoc");
        else
            fullPath = m_resolvePagePath(page_name);

        return base + QStringLiteral("/page/") + fullPath;
    }
    return export_html(page_name, isJournalPage);
}
