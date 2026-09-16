/* BlockEditor.cpp — Block-level editing operations extracted from PageStore.
 *
 * All FFI calls are ported verbatim from the original PageStore methods.
 * Page state is read through the PageStore* parameter; block-list-model
 * updates are applied directly via m_blockListModel.
 */

#include "BlockEditor.h"
#include "PageStore.h"

#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonArray>

#include "../ffi/ffi_raii.h"

/* ================================================================== */
/*  Constructor                                                        */
/* ================================================================== */

BlockEditor::BlockEditor(const BridgeContext &ctx, BlockListModel *model,
                         const PagePathResolver &pathResolver)
    : m_ctx(ctx)
    , m_blockListModel(model)
    , m_pathResolver(pathResolver)
{
}

/* ================================================================== */
/*  Block editing                                                      */
/* ================================================================== */

void BlockEditor::save_block(int index, const QString &raw_text,
                             QObject *signalTarget, PageStore *pageStore)
{
    save_block_range(index, 1, raw_text, signalTarget, pageStore);
}

void BlockEditor::save_block_range(int start_index, int count,
                                   const QString &raw_text,
                                   QObject *signalTarget, PageStore *pageStore)
{
    if (start_index < 0 || count < 0) return;

    const QString pagePath = m_pathResolver.currentRelative(
        pageStore->currentPageName(), pageStore->currentPageGroupPath(),
        pageStore->currentPageFullPath(), pageStore->isJournalPage());
    if (pagePath.isEmpty()) return;

    int rc = notes_core_page_save_block(
        m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath), qstrToFFI(pagePath),
        start_index, count, qstrToFFI(raw_text),
        m_ctx.dropComments() ? 1 : 0);

    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to save block"));
        return;
    }

    /* Optimistic update: read back, parse, update UI immediately */
    char *source = notes_core_page_get_source(
        qstrToFFI(m_ctx.notesPath), qstrToFFI(pagePath));
    QString content = ffiStringToQString(source);
    if (!content.isEmpty()) {
        char *blocks = notes_core_page_parse_and_render_blocks_json(
            content.toUtf8().constData(),
            qstrToFFI(m_ctx.notesPath),
            m_ctx.dropComments() ? 1 : 0,
            m_ctx.themeColorsJson().isEmpty() ? nullptr
                                              : qstrToFFI(m_ctx.themeColorsJson()),
            qstrToFFI(m_ctx.buildOptionsJson()));
        QVariantList blockList = PageStore::jsonArrayToQStringVariantList(
            ffiStringToQString(blocks));
        if (m_blockListModel) {
            m_blockListModel->setBlocks(blockList);
        }
    }

    /* Background reload for full refresh */
    if (signalTarget)
        pageStore->load_page(pagePath, signalTarget);
}

void BlockEditor::insert_link_at_cursor(int block_idx, int /*cursor_pos*/,
                                        const QString &target,
                                        QObject *signalTarget, PageStore *pageStore)
{
    if (block_idx < 0) return;

    const QString pagePath = m_pathResolver.currentRelative(
        pageStore->currentPageName(), pageStore->currentPageGroupPath(),
        pageStore->currentPageFullPath(), pageStore->isJournalPage());
    if (pagePath.isEmpty()) return;

    /* Read current source */
    char *src = notes_core_page_get_source(
        qstrToFFI(m_ctx.notesPath), qstrToFFI(pagePath));
    QString content = ffiStringToQString(src);
    if (content.isEmpty()) return;

    /* Parse blocks to find the target block */
    char *blocksJson = notes_core_parse_blocks_json(
        qstrToFFI(content), m_ctx.dropComments() ? 1 : 0);
    QJsonDocument doc = QJsonDocument::fromJson(
        ffiStringToQString(blocksJson).toUtf8());
    QJsonArray arr = doc.array();

    if (block_idx >= arr.size()) return;

    /* Build xref link */
    const QString xref = QStringLiteral("xref:%1.adoc[%1]").arg(target);

    /* Get block's text and append the link */
    QJsonObject blockObj = arr[block_idx].toObject();
    QString existingText = blockObj[QStringLiteral("text")].toString();
    if (existingText.isEmpty())
        existingText = blockObj[QStringLiteral("raw")].toString();
    QString newRaw = existingText + QLatin1Char(' ') + xref;

    /* Replace the block */
    int rc = notes_core_page_save_block(
        m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath), qstrToFFI(pagePath),
        block_idx, 1, qstrToFFI(newRaw), m_ctx.dropComments() ? 1 : 0);

    if (rc >= 0)
        pageStore->load_page(pagePath, signalTarget);
}

void BlockEditor::toggle_checkbox(int block_index, const QString &item_path,
                                  QObject *signalTarget, PageStore *pageStore)
{
    if (block_index < 0) return;

    const QString pagePath = m_pathResolver.currentRelative(
        pageStore->currentPageName(), pageStore->currentPageGroupPath(),
        pageStore->currentPageFullPath(), pageStore->isJournalPage());
    if (pagePath.isEmpty()) return;

    int rc = notes_core_page_toggle_checkbox(
        m_ctx.rawConn(), qstrToFFI(m_ctx.notesPath), qstrToFFI(pagePath),
        block_index, qstrToFFI(item_path));

    if (rc < 0) {
        m_ctx.reportError(QStringLiteral("Failed to toggle checkbox"));
        return;
    }
    if (m_blockListModel) {
        m_blockListModel->toggleCheckbox(block_index, item_path);
    }
    pageStore->load_page(pagePath, signalTarget);
}
