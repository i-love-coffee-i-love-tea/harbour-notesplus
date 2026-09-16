/* PagePathResolver — stateless helper for resolving page names to relative
 * paths within a notes directory.
 *
 * Extracted from PageStore so that path-logic can be tested and reused
 * independently of page state.  All page state is passed as parameters
 * rather than stored.
 */

#ifndef PAGEPATHRESOLVER_H
#define PAGEPATHRESOLVER_H

#include <QString>

class PagePathResolver
{
public:
    explicit PagePathResolver(const QString &notesPath);

    /* Resolve a page name to a relative path within the notes directory.
     * Checks journal special-case, slash-containing paths, scoped-under-
     * current-group with file-existence probe, and root fallback.          */
    QString resolve(const QString &name,
                    const QString &currentGroupPath) const;

    /* Derive the current page's relative path from its constituent state
     * fields.  Returns "journal.adoc" for journal pages, the stored full
     * path when available, or a composed group/name path as fallback.      */
    QString currentRelative(const QString &pageName,
                            const QString &groupPath,
                            const QString &fullPath,
                            bool isJournal) const;

    /* Extract a human-readable title from a file path or name by stripping
     * the directory component and ".adoc" suffix.                           */
    static QString extractTitle(const QString &name);

private:
    QString m_notesPath;
};

#endif /* PAGEPATHRESOLVER_H */
