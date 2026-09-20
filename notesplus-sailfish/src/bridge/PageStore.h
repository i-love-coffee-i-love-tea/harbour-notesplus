/* PageStore.h — Page CRUD and current page state for Notes++.
 *
 * Block editing (save_block, save_block_range, insert_link_at_cursor,
 * toggle_checkbox) has been extracted to BlockEditor.
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
#include "PagePathResolver.h"

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

    /* ---- Page operations ---- */

    void    load_page(const QString &name, QObject *signalTarget);
    void    append_to_current_page(const QString &text, bool is_task,
                                   QObject *signalTarget);
    QString get_page_source(const QString &name);
    void    save_page_source(const QString &name, const QString &content,
                             QObject *signalTarget);
    void    create_page(const QString &name, const QString &color,
                        QObject *signalTarget);
    bool    set_page_color(const QString &name, const QString &color);
    void    delete_page(const QString &name, QObject *signalTarget);
    bool    rename_page(const QString &old_path, const QString &new_title,
                        QObject *signalTarget);
    void    navigate_to_page(const QString &name, QObject *signalTarget);
    QString rebuild_index();

    /* ---- Path resolver ---- */

    const PagePathResolver &pathResolver() const { return m_pathResolver; }
    void setNotesPath(const QString &newPath)
    {
        m_pathResolver = PagePathResolver(newPath);
    }

    /* ---- Async poll ---- */

    PollResult poll_results();

    /* ---- Current page state ---- */

    QString      currentPageName()      const { return m_currentPageName; }
    QString      currentPageGroupPath() const { return m_currentPageGroupPath; }
    QString      currentPageFullPath()  const { return m_currentPageFullPath; }
    QString      currentPageFilePath()  const { return m_currentPageFilePath; }
    QVariantList currentBlocks()        const { return m_currentBlocks; }
    bool         isJournalPage()        const { return m_isJournalPage; }
    int          blocksVersion()        const { return m_blocksVersion; }
    bool         isLoading()            const { return m_loading; }

    void updatePagePaths(const QString &groupPath, const QString &fullPath,
                         const QString &filePath);

    /* ---- Callback ---- */

    void setRebuildTreeCallback(std::function<void()> cb)
    { m_rebuildTreeCallback = std::move(cb); }

    /* ---- Static helpers ---- */

    static QVariantList jsonArrayToQStringVariantList(const QString &jsonStr);

private:
    const BridgeContext &m_ctx;
    BlockListModel      *m_blockListModel = nullptr;
    PagePathResolver     m_pathResolver;

    /* ---- Current page state ---- */
    QString      m_currentPageName;
    QString      m_currentPageGroupPath;
    QString      m_currentPageFullPath;
    QString      m_currentPageFilePath;
    QVariantList m_currentBlocks;
    bool         m_isJournalPage = false;
    int          m_blocksVersion = 0;
    bool         m_loading = false;

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
