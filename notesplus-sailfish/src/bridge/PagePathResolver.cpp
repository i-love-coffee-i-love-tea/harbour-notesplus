/* PagePathResolver — stateless helper for resolving page names to relative
 * paths within a notes directory.
 *
 * Logic copied verbatim from PageStore.cpp (resolvePagePath,
 * currentPageRelativePath, extractTitle).
 */

#include "PagePathResolver.h"

#include <QFile>

/* ================================================================== */
/*  Constructor                                                        */
/* ================================================================== */

PagePathResolver::PagePathResolver(const QString &notesPath)
    : m_notesPath(notesPath)
{
}

/* ================================================================== */
/*  Path resolution                                                    */
/* ================================================================== */

QString PagePathResolver::resolve(const QString &name,
                                  const QString &currentGroupPath) const
{
    if (name.compare(QLatin1String("journal.adoc"), Qt::CaseInsensitive) == 0
        || name.compare(QLatin1String("Journal"), Qt::CaseInsensitive) == 0)
        return QStringLiteral("journal.adoc");

    /* If it already contains a slash, assume it is a relative path */
    if (name.contains(QLatin1Char('/'))) {
        if (!name.endsWith(QLatin1String(".adoc")))
            return name + QLatin1String(".adoc");
        return name;
    }

    /* Try scoped under current group */
    if (!currentGroupPath.isEmpty()) {
        QString scoped = currentGroupPath + QLatin1Char('/') + name;
        if (!scoped.endsWith(QLatin1String(".adoc")))
            scoped += QLatin1String(".adoc");
        /* Verify file exists */
        QString absPath = m_notesPath + QLatin1Char('/') + scoped;
        if (QFile::exists(absPath))
            return scoped;
    }

    /* Fallback: name at root */
    if (!name.endsWith(QLatin1String(".adoc")))
        return name + QLatin1String(".adoc");
    return name;
}

QString PagePathResolver::currentRelative(const QString &pageName,
                                          const QString &groupPath,
                                          const QString &fullPath,
                                          bool isJournal) const
{
    if (isJournal
        || pageName.compare(QLatin1String("Journal"),
                            Qt::CaseInsensitive) == 0)
        return QStringLiteral("journal.adoc");
    if (!fullPath.isEmpty())
        return fullPath;
    if (!groupPath.isEmpty() && !pageName.isEmpty()) {
        QString p = groupPath + QLatin1Char('/') + pageName;
        if (!p.endsWith(QLatin1String(".adoc")))
            p += QLatin1String(".adoc");
        return p;
    }
    if (!pageName.isEmpty()) {
        if (!pageName.endsWith(QLatin1String(".adoc")))
            return pageName + QLatin1String(".adoc");
        return pageName;
    }
    return QString();
}

/* ================================================================== */
/*  Static helpers                                                     */
/* ================================================================== */

/// Extract a title from a file path or name (strip directory + .adoc suffix).
QString PagePathResolver::extractTitle(const QString &name)
{
    QString t = name.section(QLatin1Char('/'), -1);
    if (t.endsWith(QLatin1String(".adoc")))
        t.chop(5);
    return t;
}
