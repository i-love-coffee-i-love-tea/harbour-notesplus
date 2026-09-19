/* GroupManager.cpp — Group CRUD, display depth, sort preferences. */

#include "GroupManager.h"

#include "../ffi/ffi_raii.h"

GroupManager::GroupManager(const BridgeContext &ctx)
    : m_ctx(ctx)
{
}

void GroupManager::set_rebuild_tree_callback(std::function<void()> cb)
{
    m_rebuildTreeCallback = std::move(cb);
}

/* ================================================================== */
/*  Group CRUD                                                         */
/* ================================================================== */

bool GroupManager::create_group(const QString &parent_path, const QString &name)
{
    void *conn = m_ctx.rawConn();
    if (!conn) return false;

    int rc = notes_core_group_create(
        conn, qstrToFFI(m_ctx.notesPath),
        qstrToFFI(parent_path), qstrToFFI(name));
    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to create group: %1").arg(name));
        return false;
    }
    if (m_rebuildTreeCallback) m_rebuildTreeCallback();
    return true;
}

bool GroupManager::rename_group(const QString &old_path, const QString &new_name)
{
    void *conn = m_ctx.rawConn();
    if (!conn) return false;

    int rc = notes_core_group_rename(
        conn, qstrToFFI(m_ctx.notesPath),
        qstrToFFI(old_path), qstrToFFI(new_name));
    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to rename group"));
        return false;
    }
    if (m_rebuildTreeCallback) m_rebuildTreeCallback();
    return true;
}

bool GroupManager::delete_group(const QString &path, bool recursive)
{
    void *conn = m_ctx.rawConn();
    if (!conn) return false;

    int rc = notes_core_group_delete(
        conn, qstrToFFI(m_ctx.notesPath),
        qstrToFFI(path), recursive ? 1 : 0);
    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to delete group"));
        return false;
    }
    if (m_rebuildTreeCallback) m_rebuildTreeCallback();
    return true;
}

/* ================================================================== */
/*  Page move                                                          */
/* ================================================================== */

MovePageResult GroupManager::move_page_to_group(
    const QString &page_full_path,
    const QString &target_group,
    const QString &current_page_full_path)
{
    MovePageResult result;
    void *conn = m_ctx.rawConn();
    if (!conn) return result;

    int rc = notes_core_page_move(
        conn, qstrToFFI(m_ctx.notesPath),
        qstrToFFI(page_full_path), qstrToFFI(target_group));
    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to move page"));
        return result;
    }

    result.success = true;

    /* Compute new paths from the original filename */
    int sl = page_full_path.lastIndexOf(QLatin1Char('/'));
    QString filename = (sl >= 0) ? page_full_path.mid(sl + 1)
                                 : page_full_path;
    result.newGroupPath = target_group;
    result.newFullPath  = target_group.isEmpty()
        ? filename
        : target_group + QLatin1Char('/') + filename;
    result.newFilePath  = result.newFullPath;

    /* Determine if the moved page is the currently active one */
    result.currentPageMoved = (current_page_full_path == page_full_path);

    if (m_rebuildTreeCallback) m_rebuildTreeCallback();
    return result;
}

/* ================================================================== */
/*  Display depth                                                      */
/* ================================================================== */

bool GroupManager::set_group_display_depth(int depth)
{
    if (m_groupDisplayDepth == depth)
        return false;
    m_groupDisplayDepth = depth;
    return true;
}

/* ================================================================== */
/*  Collapse toggle                                                    */
/* ================================================================== */

bool GroupManager::toggle_group_collapsed(const QString &group_path)
{
    void *conn = m_ctx.rawConn();
    if (!conn) return false;

    int rc = notes_core_group_toggle_collapsed(
        conn, qstrToFFI(group_path));
    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to toggle group collapsed state"));
        return false;
    }
    if (m_rebuildTreeCallback) m_rebuildTreeCallback();
    return true;
}

/* ================================================================== */
/*  Sort preferences                                                   */
/* ================================================================== */

bool GroupManager::set_group_note_sort(const QString &group_path,
                                       const QString &note_sort)
{
    void *conn = m_ctx.rawConn();
    if (!conn) return false;

    /* Convert sort string to int: "newest"->0, "oldest"->1, "alphabetical"->2 */
    int sortOrder = 0;
    if (note_sort == QLatin1String("oldest"))
        sortOrder = 1;
    else if (note_sort == QLatin1String("alphabetical"))
        sortOrder = 2;

    int rc = notes_core_group_set_note_sort(
        conn, qstrToFFI(group_path), sortOrder);
    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to set group note sort"));
        return false;
    }
    if (m_rebuildTreeCallback) m_rebuildTreeCallback();
    return true;
}

QString GroupManager::get_group_note_sort(const QString &group_path)
{
    void *conn = m_ctx.rawConn();
    if (!conn) return QStringLiteral("newest");

    int sort = notes_core_group_get_note_sort(
        conn, qstrToFFI(group_path));

    switch (sort) {
    case 1:  return QStringLiteral("oldest");
    case 2:  return QStringLiteral("alphabetical");
    default: return QStringLiteral("newest");
    }
}

/* ================================================================== */
/*  Flat groups list                                                   */
/* ================================================================== */

QString GroupManager::get_groups_json()
{
    void *conn = m_ctx.rawConn();
    if (!conn) return QStringLiteral("[]");

    char *json = notes_core_groups_flat_json(conn);
    return ffiStringToQString(json);
}
