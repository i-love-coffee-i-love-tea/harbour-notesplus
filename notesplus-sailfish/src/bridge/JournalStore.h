/* JournalStore.h — Journal-specific operations extracted from PageStore.
 *
 * Handles save_journal_block, toggle_journal_checkbox, and
 * append_to_journal.  Uses the same BridgeContext as other domain classes.
 */

#ifndef JOURNALSTORE_H
#define JOURNALSTORE_H

#include <QString>
#include <functional>

#include "BridgeContext.h"

class JournalStore
{
public:
    explicit JournalStore(const BridgeContext &ctx);

    void save_journal_block(int index, const QString &raw_text);
    void toggle_journal_checkbox(int block_index, const QString &item_path);
    void append_to_journal(const QString &text, bool is_task);

    void setRebuildCallback(std::function<void()> cb);
    void setReloadCallback(std::function<void()> cb);

private:
    const BridgeContext &m_ctx;
    std::function<void()> m_rebuildCallback;
    std::function<void()> m_reloadCallback;
};

#endif /* JOURNALSTORE_H */
