/* ExportHelper.cpp — HTML and PDF export, and open-in-browser. */

#include "ExportHelper.h"

#include <QDir>
#include <QDirIterator>
#include <QFileInfo>
#include <QJsonDocument>
#include <QJsonArray>
#include <QJsonObject>
#include <QMarginsF>
#include <QMutex>
#include <QMutexLocker>
#include <QPageLayout>
#include <QPageSize>
#include <QPdfWriter>
#include <QRegularExpression>
#include <QSizeF>
#include <QStandardPaths>
#include <QTextBlock>
#include <QTextCursor>
#include <QTextDocument>
#include <QTextFragment>
#include <QTextImageFormat>
#include <QUrl>
#include <cstdlib>

#include "../ffi/ffi_raii.h"

static QString s_notesPath;
static QMutex s_pathMutex;

static const char *s_pdfPrintCss =
    "body {\n"
    "    font-family: 'Liberation Sans', 'DejaVu Sans', 'Droid Sans', sans-serif;\n"
    "    font-size: 10pt;\n"
    "    color: #24292f;\n"
    "}\n"
    ".document-header {\n"
    "    border-bottom: 2px solid #24292f;\n"
    "    padding-bottom: 8px;\n"
    "    margin-bottom: 20px;\n"
    "}\n"
    ".document-title {\n"
    "    font-size: 24pt;\n"
    "    font-weight: bold;\n"
    "    color: #111111;\n"
    "    margin-top: 0px;\n"
    "    margin-bottom: 6px;\n"
    "}\n"
    "h1, h2, h3, h4, h5, h6 {\n"
    "    color: #111111;\n"
    "    font-weight: bold;\n"
    "}\n"
    "h1 {\n"
    "    font-size: 18pt;\n"
    "    margin-top: 18px;\n"
    "    margin-bottom: 8px;\n"
    "    border-bottom: 1px solid #d0d7de;\n"
    "    padding-bottom: 4px;\n"
    "}\n"
    "h2 {\n"
    "    font-size: 14pt;\n"
    "    margin-top: 16px;\n"
    "    margin-bottom: 8px;\n"
    "    border-bottom: 1px solid #e1e4e8;\n"
    "    padding-bottom: 3px;\n"
    "}\n"
    "h3 {\n"
    "    font-size: 12pt;\n"
    "    margin-top: 12px;\n"
    "    margin-bottom: 6px;\n"
    "}\n"
    "h4, h5, h6 {\n"
    "    font-size: 10pt;\n"
    "    margin-top: 10px;\n"
    "    margin-bottom: 4px;\n"
    "}\n"
    "p {\n"
    "    margin-top: 0px;\n"
    "    margin-bottom: 8px;\n"
    "}\n"
    "pre, .listingblock pre, .literalblock pre {\n"
    "    font-family: 'Liberation Mono', 'DejaVu Sans Mono', monospace;\n"
    "    font-size: 8.5pt;\n"
    "    background-color: #f6f8fa;\n"
    "    color: #24292f;\n"
    "    padding: 8px 10px;\n"
    "    margin-top: 6px;\n"
    "    margin-bottom: 12px;\n"
    "    border: 1px solid #d0d7de;\n"
    "}\n"
    "pre code {\n"
    "    background-color: transparent;\n"
    "    color: #24292f;\n"
    "    padding: 0;\n"
    "}\n"
    "code {\n"
    "    font-family: 'Liberation Mono', 'DejaVu Sans Mono', monospace;\n"
    "    font-size: 8.5pt;\n"
    "    background-color: #f1f3f5;\n"
    "    color: #24292f;\n"
    "    padding: 2px 4px;\n"
    "}\n"
    ".listingblock .title, .literalblock .title, .table-title, .example-title, .sidebar-title {\n"
    "    font-size: 9pt;\n"
    "    font-weight: bold;\n"
    "    color: #24292f;\n"
    "    margin-bottom: 4px;\n"
    "}\n"
    "table {\n"
    "    border-collapse: collapse;\n"
    "    width: 100%;\n"
    "    margin-top: 8px;\n"
    "    margin-bottom: 14px;\n"
    "}\n"
    "th, td {\n"
    "    border: 1px solid #d0d7de;\n"
    "    padding: 6px 8px;\n"
    "    font-size: 9pt;\n"
    "}\n"
    "th {\n"
    "    background-color: #f6f8fa;\n"
    "    font-weight: bold;\n"
    "    color: #24292f;\n"
    "}\n"
    ".admonitionblock {\n"
    "    margin-top: 10px;\n"
    "    margin-bottom: 12px;\n"
    "    padding: 10px;\n"
    "    border-left: 4px solid #0284c7;\n"
    "    background-color: #f0f9ff;\n"
    "}\n"
    ".admonitionblock.note {\n"
    "    border-left: 4px solid #0284c7;\n"
    "    background-color: #f0f9ff;\n"
    "}\n"
    ".admonitionblock.tip {\n"
    "    border-left: 4px solid #f0ad4e;\n"
    "    background-color: #fefce8;\n"
    "}\n"
    ".admonitionblock.warning {\n"
    "    border-left: 4px solid #d9534f;\n"
    "    background-color: #fee2e2;\n"
    "}\n"
    ".admonitionblock.important {\n"
    "    border-left: 4px solid #ef4444;\n"
    "    background-color: #fef2f2;\n"
    "}\n"
    ".admonitionblock.caution {\n"
    "    border-left: 4px solid #f97316;\n"
    "    background-color: #fff7ed;\n"
    "}\n"
    ".admonition-title {\n"
    "    font-weight: bold;\n"
    "    font-size: 10pt;\n"
    "    color: #1f2328;\n"
    "}\n"
    "blockquote {\n"
    "    margin-left: 12px;\n"
    "    margin-right: 0px;\n"
    "    padding: 6px 12px;\n"
    "    border-left: 3px solid #8b949e;\n"
    "    background-color: #f6f8fa;\n"
    "    color: #57606a;\n"
    "}\n"
    ".sidebarblock {\n"
    "    background-color: #f6f8fa;\n"
    "    border: 1px solid #d0d7de;\n"
    "    padding: 10px 14px;\n"
    "    margin-top: 10px;\n"
    "    margin-bottom: 14px;\n"
    "}\n"
    ".exampleblock {\n"
    "    background-color: #f8fafc;\n"
    "    border: 1px solid #e2e8f0;\n"
    "    padding: 10px 14px;\n"
    "    margin-top: 10px;\n"
    "    margin-bottom: 14px;\n"
    "}\n"
    ".toc {\n"
    "    background-color: #f6f8fa;\n"
    "    border: 1px solid #d0d7de;\n"
    "    padding: 10px 14px;\n"
    "    margin-top: 10px;\n"
    "    margin-bottom: 16px;\n"
    "}\n"
    ".toctitle {\n"
    "    font-size: 11pt;\n"
    "    font-weight: bold;\n"
    "    color: #1f2328;\n"
    "    margin-bottom: 6px;\n"
    "}\n"
    ".toc ul {\n"
    "    list-style: none;\n"
    "    list-style-type: none;\n"
    "    margin-top: 0px;\n"
    "    margin-bottom: 4px;\n"
    "    padding-left: 16px;\n"
    "}\n"
    ".toc ul.sectlevel1 {\n"
    "    padding-left: 0px;\n"
    "}\n"
    ".toc li {\n"
    "    list-style: none;\n"
    "    list-style-type: none;\n"
    "    margin-bottom: 3px;\n"
    "    font-size: 9.5pt;\n"
    "}\n"
    "ul, ol {\n"
    "    margin-top: 0px;\n"
    "    margin-bottom: 8px;\n"
    "    padding-left: 20px;\n"
    "}\n"
    "li {\n"
    "    margin-bottom: 3px;\n"
    "}\n"
    "a {\n"
    "    color: #0969da;\n"
    "    text-decoration: none;\n"
    "}\n"
    ".document-footer {\n"
    "    border-top: 1px solid #d0d7de;\n"
    "    margin-top: 24px;\n"
    "    padding-top: 12px;\n"
    "    font-size: 8pt;\n"
    "    color: #6e7781;\n"
    "    text-align: center;\n"
    "}\n"
    "hr {\n"
    "    border: 0px;\n"
    "    border-top: 1px solid #d0d7de;\n"
    "    margin: 14px 0px;\n"
    "}\n";

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
    QMutexLocker locker(&s_pathMutex);
    s_notesPath = ctx.notesPath;
}

bool ExportHelper::render_page_to_pdf_static(const QString &notesPath, const QString &page_path, const QString &output_path)
{
    if (notesPath.isEmpty() || page_path.isEmpty() || output_path.isEmpty())
        return false;

    QFileInfo outInfo(output_path);
    QDir().mkpath(outInfo.absolutePath());

    char *result = notes_core_render_page_html5(qstrToFFI(notesPath), qstrToFFI(page_path));
    QString html = ffiStringToQString(result);
    if (html.isEmpty())
        return false;

    // Replace browser-specific <style> block with print-optimized CSS tailored for QTextDocument
    static const QRegularExpression styleRegex(QStringLiteral("<style[^>]*>.*?</style>"),
                                               QRegularExpression::DotMatchesEverythingOption);
    html.replace(styleRegex, QStringLiteral("<style type=\"text/css\">\n") + QString::fromLatin1(s_pdfPrintCss) + QStringLiteral("\n</style>"));

    // Map dark syntax highlighting colors to high-contrast print colors
    static const struct {
        const char *from;
        const char *to;
    } s_colorMap[] = {
        {"#c0c5ce", "#24292f"},
        {"#8fa1b3", "#0550ae"},
        {"#96b5b4", "#0969da"},
        {"#a3be8c", "#116329"},
        {"#b48ead", "#8250df"},
        {"#bf616a", "#cf222e"},
        {"#d08770", "#953800"},
        {"#65737e", "#57606a"},
    };
    for (size_t i = 0; i < sizeof(s_colorMap) / sizeof(s_colorMap[0]); ++i) {
        html.replace(QString::fromLatin1(s_colorMap[i].from),
                     QString::fromLatin1(s_colorMap[i].to),
                     Qt::CaseInsensitive);
    }

    // Convert inline SVG tags to base64 embedded images so QTextDocument can render them
    static const QRegularExpression svgRegex(QStringLiteral("<svg\\b[^>]*>.*?</svg>"),
                                             QRegularExpression::DotMatchesEverythingOption);
    QRegularExpressionMatch match = svgRegex.match(html);
    while (match.hasMatch()) {
        QString svgText = match.captured(0);
        QString b64 = QString::fromLatin1(svgText.toUtf8().toBase64());
        QString imgTag = QStringLiteral("<img src=\"data:image/svg+xml;base64,") + b64 + QStringLiteral("\">");
        html.replace(match.capturedStart(0), match.capturedLength(0), imgTag);
        match = svgRegex.match(html);
    }

    QPdfWriter writer(output_path);
    writer.setPageSize(QPageSize(QPageSize::A4));
    writer.setResolution(300);

    QPageLayout layout = writer.pageLayout();
    layout.setUnits(QPageLayout::Millimeter);
    layout.setMargins(QMarginsF(15.0, 15.0, 15.0, 15.0));
    layout.setPageSize(QPageSize(QPageSize::A4));
    layout.setOrientation(QPageLayout::Portrait);
    writer.setPageLayout(layout);

    QTextDocument doc;
    doc.setBaseUrl(QUrl::fromLocalFile(notesPath + QStringLiteral("/")));
    doc.setDefaultStyleSheet(QString::fromLatin1(s_pdfPrintCss));
    QSizeF paintSize = QSizeF(layout.paintRectPixels(writer.resolution()).size());
    doc.setPageSize(paintSize);
    doc.setHtml(html);

    // Scale down any images exceeding the printable page width
    const qreal maxW = paintSize.width();
    for (QTextBlock block = doc.begin(); block != doc.end(); block = block.next()) {
        for (QTextBlock::iterator it = block.begin(); !it.atEnd(); ++it) {
            QTextFragment frag = it.fragment();
            if (frag.isValid()) {
                QTextCharFormat fmt = frag.charFormat();
                if (fmt.isImageFormat()) {
                    QTextImageFormat imgFmt = fmt.toImageFormat();
                    QVariant res = doc.resource(QTextDocument::ImageResource, QUrl(imgFmt.name()));
                    QImage loadedImg = qvariant_cast<QImage>(res);
                    qreal origW = imgFmt.width() > 0 ? imgFmt.width() : (loadedImg.isNull() ? 0 : loadedImg.width());
                    qreal origH = imgFmt.height() > 0 ? imgFmt.height() : (loadedImg.isNull() ? 0 : loadedImg.height());
                    if (origW > maxW && origW > 0) {
                        qreal scale = maxW / origW;
                        imgFmt.setWidth(maxW);
                        if (origH > 0) imgFmt.setHeight(origH * scale);
                        QTextCursor cursor(&doc);
                        cursor.setPosition(frag.position());
                        cursor.setPosition(frag.position() + frag.length(), QTextCursor::KeepAnchor);
                        cursor.setCharFormat(imgFmt);
                    }
                }
            }
        }
    }

    doc.print(&writer);
    return true;
}

int ExportHelper::pdfExportCallback(const char *note_rel_path, const char *out_pdf_path)
{
    if (!note_rel_path || !out_pdf_path)
        return -1;

    QString notesPath;
    {
        QMutexLocker locker(&s_pathMutex);
        notesPath = s_notesPath;
    }

    if (notesPath.isEmpty())
        return -1;

    const QString relPath = QString::fromUtf8(note_rel_path);
    const QString outPath = QString::fromUtf8(out_pdf_path);

    bool ok = render_page_to_pdf_static(notesPath, relPath, outPath);
    return ok ? 0 : -1;
}

bool ExportHelper::render_page_to_pdf(const QString &page_path, const QString &output_path)
{
    return render_page_to_pdf_static(m_ctx.notesPath, page_path, output_path);
}

QString ExportHelper::export_pdf(const QString &page_name, bool isJournalPage)
{
    QString fullPath;
    if (isJournalPage
        || page_name.compare(QLatin1String("Journal"), Qt::CaseInsensitive) == 0)
        fullPath = QStringLiteral("journal.adoc");
    else
        fullPath = m_resolvePagePath(page_name);

    const QString home = QString::fromLocal8Bit(qgetenv("HOME"));
    if (home.isEmpty()) return QString();
    const QString exportDir = home + QStringLiteral("/Documents/Notes++ Exports");
    QDir().mkpath(exportDir);

    const QString title = extractTitle(page_name);
    const QString outputPath = exportDir + QLatin1Char('/') + title + QLatin1String(".pdf");

    bool ok = render_page_to_pdf(fullPath, outputPath);
    if (!ok) {
        m_ctx.reportError(QStringLiteral("PDF export failed"));
        return QString();
    }
    return outputPath;
}

QString ExportHelper::export_all_pdf()
{
    const QString home = QString::fromLocal8Bit(qgetenv("HOME"));
    if (home.isEmpty()) return QStringLiteral("[]");
    const QString exportDir = home + QStringLiteral("/Documents/Notes++ Exports");
    QDir().mkpath(exportDir);

    QJsonArray pathsArray;
    QDirIterator it(m_ctx.notesPath, QStringList() << QStringLiteral("*.adoc"), QDir::Files, QDirIterator::Subdirectories);
    while (it.hasNext()) {
        it.next();
        const QString absFilePath = it.filePath();
        QString relPath = absFilePath.mid(m_ctx.notesPath.length());
        if (relPath.startsWith(QLatin1Char('/')))
            relPath = relPath.mid(1);

        const QString fileName = it.fileName();
        const QString title = extractTitle(fileName);
        const QString outputPath = exportDir + QLatin1Char('/') + title + QLatin1String(".pdf");

        if (render_page_to_pdf(relPath, outputPath)) {
            pathsArray.append(outputPath);
        }
    }

    return QString::fromUtf8(QJsonDocument(pathsArray).toJson(QJsonDocument::Compact));
}

QString ExportHelper::get_pdf_export_path(const QString &page_name, bool isJournalPage)
{
    const QString home = QString::fromLocal8Bit(qgetenv("HOME"));
    if (home.isEmpty()) return QString();
    const QString exportDir = home + QStringLiteral("/Documents/Notes++ Exports");
    QString title;
    if (isJournalPage
        || page_name.compare(QLatin1String("Journal"), Qt::CaseInsensitive) == 0)
        title = QStringLiteral("journal");
    else
        title = extractTitle(page_name);
    return exportDir + QLatin1Char('/') + title + QLatin1String(".pdf");
}

bool ExportHelper::pdf_export_exists(const QString &page_name, bool isJournalPage)
{
    const QString targetPath = get_pdf_export_path(page_name, isJournalPage);
    return !targetPath.isEmpty() && QFileInfo::exists(targetPath);
}

bool ExportHelper::any_pdf_export_exists()
{
    const QString home = QString::fromLocal8Bit(qgetenv("HOME"));
    if (home.isEmpty()) return false;
    const QString exportDir = home + QStringLiteral("/Documents/Notes++ Exports");
    QDir dir(exportDir);
    if (!dir.exists()) return false;
    QStringList pdfs = dir.entryList(QStringList() << QStringLiteral("*.pdf"), QDir::Files);
    return !pdfs.isEmpty();
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
