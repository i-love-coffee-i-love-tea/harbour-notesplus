/* GroupManager.h — Group CRUD, display depth, sort preferences.
 *
 * Pure domain class (not a QObject). Manages group operations via FFI.
 * The Facade is responsible for notifying UI / rebuilding tree based on results.
 */

#ifndef GROUPMANAGER_H
#define GROUPMANAGER_H

#include <QString>
#include <functional>

#include "BridgeContext.h"

struct MovePageResult {
    bool    success         = false;
    bool    currentPageMoved = false;
    QString newGroupPath;
    QString newFullPath;
    QString newFilePath;
};

class GroupManager
{
public:
    explicit GroupManager(const BridgeContext &ctx);

    /* Group CRUD */
    bool create_group(const QString &parent_path, const QString &name);
    bool rename_group(const QString &old_path, const QString &new_name);
    bool delete_group(const QString &path, bool recursive);

    /* Page move — returns result struct; Facade updates PageStore. */
    MovePageResult move_page_to_group(const QString &page_full_path,
                                      const QString &target_group,
                                      const QString &current_page_full_path);

    /* Display depth — returns true if changed; Facade notifies MainPageLoader. */
    bool set_group_display_depth(int depth);
    int  group_display_depth() const { return m_groupDisplayDepth; }

    /* Collapse toggle */
    bool toggle_group_collapsed(const QString &group_path);

    /* Sort preferences */
    bool    set_group_note_sort(const QString &group_path,
                                const QString &note_sort);
    QString get_group_note_sort(const QString &group_path);

    /* Flat groups JSON list */
    QString get_groups_json();

    /* Callback for tree rebuild — set by Facade */
    void set_rebuild_tree_callback(std::function<void()> cb);

private:
    const BridgeContext      &m_ctx;
    int                       m_groupDisplayDepth = 2;
    std::function<void()>     m_rebuildTreeCallback;
};

#endif /* GROUPMANAGER_H */
