/* PageStore.h — Page CRUD, block editing, journal operations, and current
 * page state for Notes++.
 *
 * Extracted from NotesBridge as a non-QObject domain class.  The facade
 * (NotesBridge) owns the PageStore, ensures DB init, and handles signal
 * emission.  Methods that trigger an async page reload take a QObject*
 * signalTarget whose "poll_results" slot is invoked when the background
 * work completes.
 */

#ifndef PAGESTORE_H
#define PAGESTORE_H

#include <QString>
#include <QVariantList>
#include <QtConcurrent/QtConcurrent>
#include <mutex>
#include <optional>
#include <functional>

#include "BlockListModel.h"
#include "BridgeContext.h"

/* Result returned by poll_results() so the facade knows what happened. */
struct PollResult {
    bool         hasResult = false;
    QVariantList blocks;
    QString      error;
};

class PageStore
{
public:
    explicit PageStore(const BridgeContext &ctx, BlockListModel *model);

    /* ---- Page operations (16) ---- */

    void    load_page(const QString &name, QObject *signalTarget);
    void    save_block(int index, const QString &raw_text);
    void    save_block_range(int start_index, int count,
                             const QString &raw_text, QObject *signalTarget);
    void    append_to_current_page(const QString &text, bool is_task,
                                   QObject *signalTarget);
    void    save_journal_block(int index, const QString &raw_text);
    void    toggle_journal_checkbox(int block_index, const QString &item_path);
    void    append_to_journal(const QString &text, bool is_task);
    QString get_page_source(const QString &name);
    void    save_page_source(const QString &name, const QString &content,
                             QObject *signalTarget);
    void    create_page(const QString &name, QObject *signalTarget);
    void    delete_page(const QString &name, QObject *signalTarget);
    bool    rename_page(const QString &old_path, const QString &new_title,
                        QObject *signalTarget);
    void    navigate_to_page(const QString &name, QObject *signalTarget);
    void    insert_link_at_cursor(int block_idx, int cursor_pos,
                                  const QString &target, QObject *signalTarget);
    void    toggle_checkbox(int block_index, const QString &item_path,
                            QObject *signalTarget);
    QString rebuild_index();

    /* ---- Path helpers ---- */

    QString resolvePagePath(const QString &name);
    QString currentPageRelativePath();

    /* ---- Async poll ---- */

    PollResult poll_results();

    /* ---- Accessors for facade ---- */

    QString      currentPageName()      const { return m_currentPageName; }
    QString      currentPageGroupPath() const { return m_currentPageGroupPath; }
    QString      currentPageFullPath()  const { return m_currentPageFullPath; }
    QString      currentPageFilePath()  const { return m_currentPageFilePath; }
    QVariantList currentBlocks()        const { return m_currentBlocks; }
    bool         isJournalPage()        const { return m_isJournalPage; }
    int          blocksVersion()        const { return m_blocksVersion; }
    bool         isLoading()            const { return m_loading; }

    /* ---- Callback (set after construction) ---- */

    void setRebuildTreeCallback(std::function<void()> cb)
    { m_rebuildTreeCallback = std::move(cb); }

    /* ---- Static helpers ---- */

    static QString      extractTitle(const QString &name);
    static QVariantList jsonArrayToQStringVariantList(const QString &jsonStr);

private:
    const BridgeContext &m_ctx;
    BlockListModel      *m_blockListModel = nullptr;

    /* ---- Current page state ---- */
    QString      m_currentPageName;
    QString      m_currentPageGroupPath;
    QString      m_currentPageFullPath;
    QString      m_currentPageFilePath;
    QVariantList m_currentBlocks;
    bool         m_isJournalPage = false;
    int          m_blocksVersion = 0;

    /* ---- Loading state ---- */
    bool m_loading = false;

    /* ---- Pending page-load result (background -> poll_results) ---- */
    struct PendingPageResult {
        QVariantList blocks;
        QString      error;
    };
    std::mutex                       m_pendingMutex;
    std::optional<PendingPageResult> m_pendingPage;

    /* ---- Callbacks ---- */
    std::function<void()> m_rebuildTreeCallback;

    Q_DISABLE_COPY(PageStore)
};

#endif /* PAGESTORE_H */
