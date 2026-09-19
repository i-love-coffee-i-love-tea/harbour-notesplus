/* BlockEditor.h — Block-level editing operations extracted from PageStore.
 *
 * Handles save_block, save_block_range, insert_link_at_cursor, and
 * toggle_checkbox.  Each method takes a PageStore* for loading pages
 * after mutation and reading current page state (currentPageName,
 * currentPageGroupPath, currentPageFullPath, isJournalPage).
 */

#ifndef BLOCKEDITOR_H
#define BLOCKEDITOR_H

#include <QString>
#include <QObject>

#include "BridgeContext.h"
#include "BlockListModel.h"
#include "PagePathResolver.h"

class PageStore;  // forward declaration

class BlockEditor
{
public:
    BlockEditor(const BridgeContext &ctx, BlockListModel *model,
                const PagePathResolver &pathResolver);

    void save_block(int index, const QString &raw_text,
                    QObject *signalTarget, PageStore *pageStore);
    void save_block_range(int start_index, int count,
                          const QString &raw_text,
                          QObject *signalTarget, PageStore *pageStore);
    void insert_link_at_cursor(int block_idx, int cursor_pos,
                               const QString &target,
                               QObject *signalTarget, PageStore *pageStore);
    void toggle_checkbox(int block_index, const QString &item_path,
                         QObject *signalTarget, PageStore *pageStore);

private:
    const BridgeContext &m_ctx;
    BlockListModel      *m_blockListModel;
    const PagePathResolver &m_pathResolver;
};

#endif /* BLOCKEDITOR_H */
